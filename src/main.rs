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

fn md_to_file_info(file: &Path) -> Result<FileData, String> {
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
                None => println!("[warning] Could not find title (empty?).")
            }
        },
        None => {
            println!("[warning] Could not find title for file '{}'. Consider After a header level 1: `# My title` at the beginning of your page.", file.to_str().unwrap());
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

    let header_content = fs::read_to_string(header_path).unwrap_or("<html><head><title>{title}</title><body>".to_owned());
    let footer_content = fs::read_to_string(footer_path).unwrap_or("</body></html>".to_owned());

    for file in files {
        create_output_file_path(&opt, &file)?;

        let file_data = md_to_file_info(&file)?;

        let assembled_content = format!("{}{}{}",
            header_content.replace("{title}", &file_data.title),
            file_data.html_content, footer_content);

        let dest_path = get_dest_file_path(&opt, &file)?;

        if fs::write(Path::new(&dest_path), assembled_content).is_err() {
            println!("[error] Couldn't not write to file: {}", dest_path.to_str().unwrap());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::env;
    use std::path::{Path, PathBuf};
    use crate::list_markdown_files;
    use crate::{Opt, get_dest_file_path};

    #[test]
    fn get_dest_file_path_works_with_absolute_path() {
        let file = Path::new("/test/full/qualified/path/my/file.md");
        let mut opt = Opt {
            input: PathBuf::new(),
            output: PathBuf::new()
        };

        opt.output.push("/test/full/qualified/path/out");
        opt.input.push("/test/full/qualified/path/");

        let result = get_dest_file_path(&opt, &file.to_path_buf()).unwrap();

        println!("{}", result.to_str().unwrap());

        assert_eq!(result, Path::new("/test/full/qualified/path/out/my/file.html").to_path_buf());
    }

    #[test]
    fn get_dest_file_path_works_with_different_output_path() {
        let file = Path::new("/test/full/qualified/path/my/file.md");
        let mut opt = Opt {
            input: PathBuf::new(),
            output: PathBuf::new()
        };

        opt.output.push("/out");
        opt.input.push("/test/full/qualified/path/");

        let result = get_dest_file_path(&opt, &file.to_path_buf()).unwrap();

        println!("{}", result.to_str().unwrap());

        assert_eq!(result, Path::new("/out/my/file.html").to_path_buf());
    }

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
