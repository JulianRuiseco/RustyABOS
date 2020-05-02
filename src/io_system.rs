use crate::abos_structs::{ABOSImmutable, ABOSMutable};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub fn export_p_matrix(abos_mutable: &ABOSMutable, abos_immutable: &ABOSImmutable) {
    let path = Path::new("./testFiles/test.grd");
    let display = path.display();

    // Open a file in write-only mode, returns `io::Result<File>`
    let mut file = match File::create(&path) {
        Err(why) => panic!("couldn't create {}: {}", display, why.description()),
        Ok(file) => file,
    };

    // Write the `LOREM_IPSUM` string to `file`, returns `io::Result<()>`

    //
    let yMin = abos_immutable.x1;
    let xMin = abos_immutable.y1;
    let yMax = abos_immutable.y2;
    let xMax = abos_immutable.x2;
    let nCol = abos_immutable.i1;
    let nRow = abos_immutable.j1;
    //
    let mut string_to_write = format!(
        "{}\r\n{}\r\n{}\r\n{}\r\n{}\r\n{}\r\n",
        yMin, xMin, yMax, xMax, nCol, nRow
    );

    for (_, row) in abos_mutable.p.row_iter().enumerate() {
        for (_, col) in row.iter().enumerate() {
            string_to_write += format!("{}\r\n", *col).as_str();
        }
    }

    match file.write_all(string_to_write.as_bytes()) {
        Err(why) => panic!("couldn't write to {}", display),
        Ok(_) => println!("successfully wrote to {}", display),
    }
}
