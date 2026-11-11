//! Opens a file, runs a command, saves as Ely does.

use std::io;

use ely_lok::Worker;

const USAGE: &str = "usage: resave <in> <out> [-p <password>] [<command> <value>...]";

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (paths, rest) = args.split_at_checked(2).ok_or_else(|| io::Error::other(USAGE))?;
    let (password, edit) = match rest {
        [flag, password, edit @ ..] if flag == "-p" => (Some(password.as_str()), edit),
        [flag, ..] if flag == "-p" => return Err(io::Error::other(USAGE)),
        edit => (None, edit),
    };
    let (input, output) = (std::path::absolute(&paths[0])?, std::path::absolute(&paths[1])?);
    let scratch = tempfile::tempdir()?;
    let mut worker = Worker::spawn(&ely_lok::install_dir()?, &scratch.path().join("profile"))?;
    worker.ready()?;
    let url = ely_lok::file_url(&input);
    let doc = match password {
        Some(password) => worker.unlock(&url, password)?,
        None => worker.load(&url)?,
    };
    if let [command, values @ ..] = edit {
        let values: Vec<&str> = values.iter().map(String::as_str).collect();
        worker.run(&doc, command, &values)?;
    }
    let format = output.extension().and_then(|ext| ext.to_str()).ok_or_else(|| io::Error::other("the output names no type"))?;
    doc.save_owned(&mut worker, &ely_lok::file_url(&output), format)?;
    worker.close_doc(doc)?;
    worker.shutdown()
}
