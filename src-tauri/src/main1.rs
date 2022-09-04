use std::process::exit;

use crate::infra::storage::Storage;

mod domains;
mod infra;

fn main() {
    let db = infra::storage::DBStorage::new("/Users/jasonjsyuan/Downloads/ankidata/TOELF/collection.anki2").unwrap();
    let cur_conf = db.get_cur_conf().unwrap();
    let notes = db.list_note().unwrap();
    println!("Hello, world!");
    print!("cur_conf: {:?}\n", cur_conf);
    print!("notes: {:#?}\n", notes);
    let sound_field = cur_conf.get_cur_model().find_sound_field();
    if sound_field == None {
        eprintln!("cannot find sound field, add sound field first!");
        exit(1);
    }
    let mut note = notes[0].clone();
    note.add_sound(sound_field.unwrap().ord).unwrap();
    db.update_note(&note).unwrap();
}
