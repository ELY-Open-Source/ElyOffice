//! Opens a file and saves it as the output's type.

use std::io;

use ely_lok::Worker;

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [input, output] = args.as_slice() else {
        return Err(io::Error::other("usage: resave <in> <out>"));
    };
    let (input, output) = (std::path::absolute(input)?, std::path::absolute(output)?);
    let scratch = tempfile::tempdir()?;
    let mut worker = Worker::spawn(&ely_lok::install_dir()?, &scratch.path().join("profile"))?;
    worker.ready()?;
    let doc = worker.load(&ely_lok::file_url(&input))?;
    let format = output.extension().and_then(|ext| ext.to_str()).ok_or_else(|| io::Error::other("the output names no type"))?;
    doc.save_as(&mut worker, &ely_lok::file_url(&output), format)?;
    worker.close_doc(doc)?;
    worker.shutdown()
}
