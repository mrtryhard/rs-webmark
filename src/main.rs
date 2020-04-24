use std::path::Path;
use std::{fs, vec::Vec};

use comrak::{markdown_to_html, ComrakOptions};

fn list_markdown_files(path: &Path) -> Vec<String> {
    let mut files = Vec::<String>::new();
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
                                        files.push(entry.path().to_str().expect("Couldn't use file path.").to_owned());
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

fn md_to_html(file: String) -> Result<String, String> {
    let content = fs::read_to_string(file);

    match content {
        Ok(content) => Ok(markdown_to_html(&content, &ComrakOptions::default())),
        Err(err) => Err(err.to_string())
    }
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
    let files = list_markdown_files(Path::new(""));
    let base_directory = "";
    let header_path = format!("{}/header.html", base_directory);
    let header_path = Path::new(&header_path);
    let footer_path = format!("{}/footer.html", base_directory);
    let footer_path = Path::new(&footer_path);

    if !header_path.exists() {
        println!("[warning] No header.html found.");
    }

    if footer_path.exists() {
        println!("[warning] No footer.html found.");
    }

    let header_content = fs::read_to_string(header_path)?;
    let footer_content = fs::read_to_string(footer_path)?;

    // TODO:
    // 1. Get working directory from args program -d <input_directory> -o <output_directory>
    // 2. Get relative paths from input_directory. e.g.: /this/is/folder/[get/this/part/only]
    //    to be able to construct same relative paths in output.
    // 3. Convert content to new content and save file. (md_to_html should do it)
    //

    for file in files {

        let html_content = md_to_html(file)?;
        let assembled = format!("{}{}{}", header_content, html_content, footer_content);

        if fs::write(Path::new(base_directory), assembled).is_err() {
            println!("Couldn't not write to file.");
        }

        // Minification of HTML?
    }

    Ok(())
}

