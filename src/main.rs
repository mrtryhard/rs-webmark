use std::path::{PathBuf, Path};
use std::{fs, vec::Vec};

use comrak::{markdown_to_html, ComrakOptions};

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

fn md_to_html(file: &Path) -> Result<String, String> {
    let content = fs::read_to_string(file);

    match content {
        Ok(content) => Ok(markdown_to_html(&content, &ComrakOptions::default())),
        Err(err) => Err(err.to_string())
    }
}

fn create_output_file_path(opt: &Opt, file: &Path) -> Result<(), Box<dyn std::error::Error + 'static>> {
    let mut p = file.to_path_buf();
    p.pop();
    let s = format!("{}/{}", opt.output.to_str().unwrap(), p.to_str().unwrap());
    fs::create_dir_all(&s)?;

    Ok(())
}

fn get_dest_file_path(opt: &Opt, file: &PathBuf) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let mut file_stripped = file.strip_prefix(&opt.input).unwrap().to_owned();
    let filename = format!("{}.html", file_stripped.file_stem().unwrap().to_str().unwrap());
    file_stripped.pop();

    Ok(opt.output.join(file_stripped).join(filename))
}

#[cfg(test)]
mod tests_listing {
    use std::env;
    use std::path::Path;
    use crate::list_markdown_files;

    #[test]
    fn list_files_recursively() {
        let directory_path = env::var("TEST_DIRECTORY_PATH").unwrap_or("".to_owned());
        let files_list = list_markdown_files(Path::new(&directory_path));

        assert_eq!(files_list.len(), 2);
    }

    #[test]
    fn on_nonexistant_dir_empty_list() {
        let files_list = list_markdown_files(Path::new("/usr/fake/path"));

        assert_eq!(files_list.len(), 0);
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
        println!("[warning] No header.html found.");
    }

    if !footer_path.exists() {
        println!("[warning] No footer.html found.");
    }

    let header_content = fs::read_to_string(header_path).unwrap_or("".to_owned());
    let footer_content = fs::read_to_string(footer_path).unwrap_or("".to_owned());

    for file in files {
        // Create the folders path (equivalent to mkdir -p <path>)
        create_output_file_path(&opt, &file)?;

        let html_content = md_to_html(&file)?;
        let assembled_content = format!("{}{}{}", header_content, html_content, footer_content);

        let dest_path = get_dest_file_path(&opt, &file)?;

        if fs::write(Path::new(&dest_path), assembled_content).is_err() {
            println!("[error] Couldn't not write to file: {}", dest_path.to_str().unwrap());
        }
    }

    Ok(())
}
