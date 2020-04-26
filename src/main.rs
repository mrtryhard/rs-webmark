use std::path::{PathBuf, Path};
use std::{fs, vec::Vec};

use comrak::{ComrakOptions, Arena};
use comrak::nodes::{NodeValue};
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

struct FileData {
    html_content: String,
    title: String
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
                                },
                                None => {}
                            }
                        }
                    },
                    Err(_) => {
                        println!("Invalid entry found.");
                    }
                }
            }
        },
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
    let page_title_node = ast_root.children().find(|item| {
        match item.data.borrow().value {
            NodeValue::Heading(ref n) => n.level == 1,
            _ => false
        }
    });

    let mut page_title = String::new();

    match page_title_node {
        Some(node) => {
            match node.first_child() {
                Some(child) => {
                    match child.data.borrow().value {
                        NodeValue::Text(ref utf8_text) => {
                            page_title = std::str::from_utf8(&utf8_text).unwrap_or("").to_owned();
                        },
                        _ => println!("[error] Couldn't extract title from file '{}'.", file.to_str().unwrap())
                    }
                },
                None => println!("[warn] Could not find title (empty?).")
            }
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
        title: page_title
    })
}

// Create the folders path (equivalent to mkdir -p <path>)
// If path has a file (has extension), it will ignore it.
fn create_output_file_path(file: &Path) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut path = file.to_path_buf();
    path.pop();
    fs::create_dir_all(&path)?;

    Ok(())
}

// From a given full input path and a destination output directory,
// obtains the resulting full output path.
fn get_dest_html_file_path(opt: &Opt, file: &PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if file.is_absolute() {
        let mut file_stripped = file
            .strip_prefix(&opt.input)
            .unwrap().to_owned();
        let filename = format!("{}.html", file_stripped.file_stem().unwrap().to_str().unwrap());
        file_stripped.pop();

        Ok(opt.output.join(file_stripped).join(filename))
    } else {
        let mut file_stripped = file
            .canonicalize()?
            .strip_prefix(&opt.input.canonicalize()?)
            .unwrap().to_owned();
        let filename = format!("{}.html", file_stripped.file_stem().unwrap().to_str().unwrap());
        file_stripped.pop();

        Ok(opt.output.join(file_stripped).join(filename))
    }
}

fn get_dest_file_path(opt: &Opt, file: &PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    if file.is_absolute() {
        let file_stripped = file
            .strip_prefix(&opt.input)
            .unwrap()
            .to_path_buf();

        Ok(opt.output.join(file_stripped))
    } else {
        let file_stripped = file.canonicalize()?
        .strip_prefix(&opt.input.canonicalize()?)
        .unwrap()
        .to_path_buf();

        Ok(opt.output.join(file_stripped))
    }
}

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let opt: Opt = Opt::from_args();

    let files = list_markdown_files(Path::new(&opt.input));

    let header_path = format!("{}/header.html", opt.input.to_str().unwrap_or("."));
    let header_path = Path::new(&header_path);
    let footer_path = format!("{}/footer.html", opt.input.to_str().unwrap_or("."));
    let footer_path = Path::new(&footer_path);

    if !header_path.exists() {
        println!("[warn] No header.html found.");
    }

    if !footer_path.exists() {
        println!("[warn] No footer.html found.");
    }

    let header_content = fs::read_to_string(header_path).unwrap_or("<html><head><title>{title}</title><body>".to_owned());
    let footer_content = fs::read_to_string(footer_path).unwrap_or("</body></html>".to_owned());

    for file in files {
        println!("[info] Processing file {}", file.to_str().unwrap());
        let dest_path = get_dest_html_file_path(&opt, &file)?;
        let _ = create_output_file_path(&dest_path)?;

        let file_data = md_to_file_data(&file)?;

        let assembled_content = format!("{}{}{}",
            header_content.replace("{title}", &file_data.title),
            file_data.html_content, footer_content);

        if fs::write(Path::new(&dest_path), assembled_content).is_err() {
            println!("[error] Couldn't not write to file: {}", dest_path.to_str().unwrap());
        }
    }

    let assets_path = format!("{}/assets.config", opt.input.to_str().unwrap_or("."));
    let assets_path = Path::new(&assets_path);

    if assets_path.exists() {
        if assets_path.is_file() {
            let assets_list: Vec<String> = fs::read_to_string(assets_path)?
                .split("\n")
                .map(|line| line.trim().to_owned())
                .collect();

            println!("[info] Copying {} assets...", assets_list.len());

            for asset in assets_list {
                let asset_path = Path::new(&asset);

                if asset_path.exists() {
                    let asset_dest = get_dest_file_path(&opt, &asset_path.to_path_buf())?;
                    let _ = create_output_file_path(asset_path);

                    match fs::copy(&asset_path, &asset_dest) {
                        Ok(_) => {},
                        Err(e) => println!("[error] Couldn't copy file '{}' to '{}'. Error: {}", asset, asset_dest.to_str().unwrap(), e.to_string())
                    }
                } else {
                    println!("[warn] Couldn't find asset '{}'. Skipping.", &asset);
                }
            }
        } else {
            println!("[warn] A folder named assets.config has been found but is not an asset descriptor file. Skipping.");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::path::{Path};
    use crate::{Opt, list_markdown_files};

    #[test]
    fn list_files_recursively() {
        let files_list = list_markdown_files(Path::new("./src/tests"));

        assert_eq!(files_list.len(), 2);
    }

    #[test]
    fn on_nonexistant_dir_empty_list() {
        let files_list = list_markdown_files(Path::new("/usr/fake/path"));

        assert_eq!(files_list.len(), 0);
    }
}
