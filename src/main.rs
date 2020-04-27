use std::path::{Path, PathBuf};
use std::{error, fs, vec::Vec};

use comrak::nodes::NodeValue;
use comrak::{Arena, ComrakOptions};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "rs-webmark", about = "A markdown-to-html website.")]
struct Opt {
    /// Input file
    #[structopt(parse(from_os_str), long = "input-directory", default_value = ".")]
    input: PathBuf,

    /// Output directory
    #[structopt(parse(from_os_str), long = "output-directory", default_value = "./out")]
    output: PathBuf,
}

#[derive(Debug)]
struct GenericError {
    message: String,
}

impl GenericError {
    fn new(error: String) -> GenericError {
        GenericError { message: error }
    }
}

impl std::fmt::Display for GenericError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "[error] {}", self.message)
    }
}

impl error::Error for GenericError {
    fn description(&self) -> &str {
        self.message.as_str()
    }
}

struct FileData {
    html_content: String,
    title: String,
}

fn list_markdown_files(path: &Path) -> Vec<PathBuf> {
    let mut files = Vec::<PathBuf>::new();
    let dir_entries = fs::read_dir(path);

    match dir_entries {
        Ok(dir) => {
            for entry in dir {
                match entry {
                    Ok(entry) => {
                        if entry.file_type().unwrap().is_dir() {
                            let mut recursively_obtained = list_markdown_files(&entry.path());
                            files.append(&mut recursively_obtained);
                        } else {
                            let path = entry.path();
                            let extension_wrapped = path.extension();

                            match extension_wrapped {
                                Some(extension) => {
                                    if extension == "md" {
                                        files.push(entry.path());
                                    }
                                }
                                None => {}
                            }
                        }
                    }
                    Err(_) => {
                        println!("Invalid entry found.");
                    }
                }
            }
        }
        Err(err) => {
            println!("Error while opening directory: {}", err.to_string());
        }
    }

    files
}

fn md_to_file_data(file: &Path) -> Result<FileData, String> {
    let arena = Arena::new();
    let file_content = fs::read_to_string(file).unwrap();

    let ast_root = comrak::parse_document(&arena, file_content.as_str(), &ComrakOptions::default());

    // Page title is the first level 1 heading we find.
    let page_title_node = ast_root
        .children()
        .find(|item| match item.data.borrow().value {
            NodeValue::Heading(ref n) => n.level == 1,
            _ => false,
        });

    let mut page_title = String::new();

    match page_title_node {
        Some(node) => match node.first_child() {
            Some(child) => match child.data.borrow().value {
                NodeValue::Text(ref utf8_text) => {
                    page_title = std::str::from_utf8(&utf8_text).unwrap_or("").to_owned();
                }
                _ => println!(
                    "[error] Couldn't extract title from file '{}'.",
                    file.to_str().unwrap()
                ),
            },
            None => println!("[warn] Could not find title (empty?)."),
        },
        None => {
            println!("[warn] Could not find title for file '{}'. Consider adding a header level 1: `# My title` at the beginning of your page.", file.to_str().unwrap());
        }
    }

    let mut output = vec![];
    if let Err(_) = comrak::format_html(&ast_root, &ComrakOptions::default(), &mut output) {
        return Err("Could not format html.".to_owned());
    }

    Ok(FileData {
        html_content: String::from_utf8(output).unwrap(),
        title: page_title,
    })
}

// Create the folders path (equivalent to mkdir -p <path>)
// file is expected to have a filename to it.
fn create_output_file_path(file: &Path) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut path = file.to_path_buf();
    path.pop();
    fs::create_dir_all(&path)?;

    Ok(())
}

// Expects all input paths to be absolutes (input directory, output directory, file)
fn destination_for_file(
    parameters: &Opt,
    file: &PathBuf,
) -> Result<PathBuf, Box<dyn std::error::Error + 'static>> {
    assert!(parameters.input.is_absolute());
    assert!(parameters.output.is_absolute());
    assert!(file.is_absolute());

    Ok(parameters
        .output
        .join(file.strip_prefix(&parameters.input)?))
}

fn read_file_string(file: &PathBuf) -> Result<String, String> {
    let path = Path::new(&file);

    if path.exists() {
        match fs::read_to_string(path) {
            Ok(content) => return Ok(content),
            Err(error) => {
                let error = format!(
                    "[error] Could not read file '{}'. Error: {}",
                    file.to_str().unwrap(),
                    error.to_string()
                );
                println!("{}", &error);
                return Err(error);
            }
        }
    }

    let error = format!("[warn] Couldn't find file '{}'", &file.to_str().unwrap());
    println!("{}", &error);
    Err(error)
}

fn assemble_file(file_data: &FileData, header: &String, footer: &String, destination: &PathBuf) {
    let assembled_content = format!(
        "{}{}{}",
        header.replace("{title}", &file_data.title),
        file_data.html_content,
        footer
    );

    if let Err(error) = fs::write(Path::new(&destination), assembled_content) {
        println!(
            "[error] Couldn't not write to file '{}'. Error: {}",
            destination.to_str().unwrap(),
            error.to_string()
        );
    }
}

// 1. Validates the input directory exists and is not a file.
// 2. Creates the base output directory.
// 3. Converts the input and output directory to absolute paths.
fn normalize_program_arguments(parameters: &Opt) -> Result<Opt, GenericError> {
    if !parameters.input.exists() {
        return Err(GenericError::new(
            "Input directory was not found.".to_owned(),
        ));
    }

    if !parameters.output.exists() {
        if let Err(error) = fs::create_dir_all(&parameters.output) {
            return Err(GenericError::new(format!(
                "Could not create output directory. Error: {}",
                error.to_string()
            )));
        }
    }

    let mut new_parameters = Opt {
        input: parameters.input.to_path_buf(),
        output: parameters.output.to_path_buf(),
    };

    match parameters.input.canonicalize() {
        Ok(path) => {
            new_parameters.input = path;
        }
        Err(error) => {
            return Err(GenericError::new(format!(
                "Could not resolve path for input directory '{}'. Error: {}",
                parameters.input.to_str().unwrap_or_default(),
                error.to_string()
            )));
        }
    }

    match parameters.output.canonicalize() {
        Ok(path) => {
            new_parameters.output = path;
        }
        Err(error) => {
            return Err(GenericError::new(format!(
                "Could not resolve path for input directory '{}'. Error: {}",
                parameters.output.to_str().unwrap_or_default(),
                error.to_string()
            )));
        }
    }

    Ok(new_parameters)
}

fn main() -> Result<(), Box<dyn error::Error + 'static>> {
    let arguments = normalize_program_arguments(&Opt::from_args())?;

    let files = list_markdown_files(Path::new(&arguments.input));

    let mut header_path = PathBuf::new();
    header_path.push("header.html");

    let header_content = read_file_string(&header_path)
    .unwrap_or("<html><head><title>{title}</title><body>".to_owned());

    let mut footer_path = PathBuf::new();
    footer_path.push("footer.html");

    let footer_content = read_file_string(&footer_path)
    .unwrap_or("</body></html>".to_owned());

    for file in files {
        println!("[info] Processing file {}", file.to_str().unwrap());

        let mut destination = destination_for_file(&arguments, &file)?;
        destination.set_extension("html");

        let file_data = md_to_file_data(&file)?;

        create_output_file_path(&destination)?;
        assemble_file(&file_data, &header_content, &footer_content, &destination);
    }

    let mut path = PathBuf::new();
    path.push(&arguments.input);
    path.push("assets.config");

    let assets: Vec<PathBuf> = read_file_string(&path)
    .unwrap_or("".to_owned())
    .split("\n")
    .skip_while(|e| e == &"")
    .map(|line| {
        let buf = Path::new(line.trim()).to_path_buf();
        buf.canonicalize().unwrap_or(buf)
    })
    .collect();

    println!("[info] Copying {} assets...", assets.len());

    for asset in &assets {
        let destination = destination_for_file(&arguments, &asset)?;

        println!("[info] Copying '{}'\n \tto '{}'.", asset.to_str().unwrap(), destination.to_str().unwrap());

        create_output_file_path(&destination)?;

        let _ = fs::copy(&asset, &destination)
        .map_err(|error| {
            println!(
                "[error] Could not copy asset '{}'. Error: ",
                error.to_string()
            );
        });
    }

    Ok(())
}
