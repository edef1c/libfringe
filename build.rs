use std::io::Command;
use std::io::fs::PathExtensions;
use std::io::fs;
use std::os;

fn main() {
  let out_dir = os::getenv("OUT_DIR").unwrap();
  let mut objects = Vec::new();

  let files = fs::readdir(&Path::new("src")).unwrap();
  let mut files = files.iter().filter(|p| p.is_file());

  for file in files {
    if let Some(filename) = file.filename_str() {
      let filepath = format!("src/{}", filename);
      let outpath;

      if let Some(basename) = eat_extension(filename, ".c") {
        outpath = format!("{}/{}.o", out_dir, basename);

        Command::new("cc").args(&[filepath.as_slice(), "-c", "-fPIC", "-o"])
                          .arg(outpath.clone())
                          .status().unwrap();
      }
      else if let Some(basename) = eat_extension(filename, ".s") {
        outpath = format!("{}/{}.o", out_dir, basename);

        Command::new("nasm").args(&[filepath.as_slice(), "-felf64", "-o"])
                            .arg(outpath.clone())
                            .status().unwrap();
      }
      else { continue }

      objects.push(outpath);
    }
  }

  Command::new("ar").args(&["crus", "libcontext.a"])
                    .args(objects.as_slice())
                    .cwd(&Path::new(&out_dir))
                    .status().unwrap();

  println!("cargo:rustc-flags=-L {} -l context:static", out_dir);
}

fn eat_extension<'a>(s: &'a str, ext: &str) -> Option<&'a str> {
  if s.ends_with(ext) {
    Some(s.slice_to(s.len() - ext.len()))
  }
  else {
    None
  }
}
