use easy_error::{format_err, Error, ResultExt};
use std::{
    cell::RefCell,
    collections::HashMap,
    env,
    fmt::Display,
    fs::{self, File},
    io::{self, Read, Seek, Write},
    path::PathBuf,
    str::FromStr,
};
use zip::write::FileOptions;

use crate::infra::{self, storage::Storage};

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Note {
    pub id: String,
    pub sfld: String,
    pub fields: Vec<String>,
}

impl Note {
    pub fn add_sound(&mut self, ord: i32) -> Result<(), Error> {
        if let Some(possible_sound) = self.fields.get(ord as usize) {
            if possible_sound.starts_with("[sound:") {
                // sound already exists, do not need to insert
                return Ok(());
            }
        }
        self.fields
            .insert(ord as usize, format!("[sound:{}.mp3]", self.sfld));
        return Ok(());
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Collection {
    work_dir: PathBuf,
    pub conf: Conf,
    pub notes: RefCell<Vec<Note>>,
    pub medias: RefCell<Vec<Media>>,
}

macro_rules! skip_none {
    ($x:expr) => {
        match $x {
            Some(x) => x,
            None => continue,
        }
    };
}

macro_rules! skip_fail {
    ($x:expr) => {
        match $x {
            Ok(x) => x,
            _error => continue,
        }
    };
}
impl Display for Collection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "name: {}, notes: {}, medias: {}",
            self.conf.cur_model.name,
            self.notes.borrow().len(),
            self.medias.borrow().len()
        )
    }
}

impl Collection {
    pub fn init_work_dir(&self) {
        let tmp = self.work_dir.clone();
        let _ = fs::remove_dir_all(&tmp);
        let _ = fs::create_dir_all(&tmp);
    }

    pub fn get_file_path(&self, fname: &str) -> PathBuf {
        let mut tmp = self.work_dir.clone();
        tmp.push(fname);
        return tmp;
    }

    pub fn import(path: &str) -> Result<Self, Error> {
        let fname = std::path::Path::new(path);
        let file = fs::File::open(&fname).context("open anki collection failed")?;

        let mut work_dir = env::temp_dir();
        work_dir.push("anki-sound-adder");

        let mut inst = Collection {
            work_dir,
            conf: Conf::default(),
            notes: RefCell::new(Vec::new()),
            medias: RefCell::new(Vec::new()),
        };
        inst.init_work_dir();

        let mut medias = Vec::new();
        let mut db_tmp_path: Option<PathBuf> = None;
        let mut archive =
            zip::ZipArchive::new(file).context("open anki collection archive failed")?;
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).context("read file failed")?;
            let fname =
                skip_none!(skip_none!(skip_none!(file.enclosed_name()).file_name()).to_str())
                    .to_string();

            // anki2 is old version data
            // only loads from anki2 if anki21 not exists
            if fname.ends_with(".anki2") {
                if db_tmp_path == None {
                    db_tmp_path = Some(inst.get_file_path(&fname));
                }
            } else if fname.ends_with(".anki21") {
                db_tmp_path = Some(inst.get_file_path(&fname));
            } else if fname == "media" {
                // build media
                type MediaMeta = HashMap<String, String>;
                let media_meta: MediaMeta = skip_fail!(serde_json::from_reader(&mut file));
                medias.reserve(media_meta.len());
                for (guid, name) in media_meta.into_iter() {
                    let media_path = inst.get_file_path(&guid);
                    medias.push(Media {
                        name: name,
                        path: media_path,
                    });
                }
            }
            let mut outfile = skip_fail!(fs::File::create(inst.get_file_path(&fname)));
            log::info!("outfile={:#?}", inst.get_file_path(&fname));
            skip_fail!(io::copy(&mut file, &mut outfile));
        }
        let db_path = db_tmp_path.ok_or(format_err!("cannot find anki data"))?;
        log::info!("dbpath={}", db_path.display());
        infra::repository::init_dbstorage(
            infra::storage::DBStorage::new(&db_path.as_path())
                .context("open anki collection db failed")?,
        );
        let db = infra::repository::get_dbstorage().lock().unwrap();
        let cur_conf = db
            .as_ref()
            .unwrap()
            .get_cur_conf()
            .context("get_cur_conf failed")
            .unwrap_or_default();
        let notes = db
            .as_ref()
            .unwrap()
            .list_notes()
            .context("list_notes failed")
            .unwrap_or_default();

        log::error!("work_dir={}", &inst.work_dir.to_string_lossy());

        inst.conf = cur_conf;
        *inst.notes.borrow_mut() = notes;
        *inst.medias.borrow_mut() = medias;
        return Ok(inst);
    }

    pub fn add_media(&self, media: Media) -> Result<(), Error> {
        let media_path = media.path.to_owned();
        let mut dst_path = self.work_dir.clone();
        dst_path.push(self.medias.borrow().len().to_string());
        fs::copy(
            &media_path,
            self.get_file_path(&self.medias.borrow().len().to_string()),
        )
        .context("copy media file failed")?;
        self.medias.borrow_mut().push(media);
        return Ok(());
    }

    fn build_media_meta(&self) -> Result<(), Error> {
        // build media
        type MediaMeta = HashMap<String, String>;
        let mut media_meta = MediaMeta::new();
        media_meta.reserve(self.medias.borrow().len());
        for (idx, media) in self.medias.borrow().iter().enumerate() {
            media_meta.insert(idx.to_string(), media.name.to_string());
        }
        let file =
            fs::File::create(self.get_file_path("media")).context("create media file failed")?;
        serde_json::to_writer(&file, &media_meta).context("write media file failed")?;
        return Ok(());
    }

    pub fn export<T: Write + Seek>(&self, writer: T) -> Result<(), Error> {
        // build media
        self.build_media_meta().context("build media meta failed")?;
        let mut zip = zip::ZipWriter::new(writer);
        let options = FileOptions::default()
            .compression_method(zip::CompressionMethod::Stored)
            .unix_permissions(0o755);

        let mut buffer = Vec::new();
        for entry in fs::read_dir(&self.work_dir).context("read work_dir fail")? {
            let entry = entry.context("get entry failed")?;
            let path = entry.path();
            let name = path
                .file_name()
                .ok_or(format_err!("get file name failed"))?;

            if path.is_file() {
                zip.start_file(name.to_string_lossy(), options)
                    .context("start zipfile failed")?;
                let mut f = File::open(path).context("open file failed")?;
                f.read_to_end(&mut buffer).context("read file failed")?;
                zip.write_all(buffer.as_ref())
                    .context("write zipfile failed")?;
                buffer.clear();
            }
        }
        zip.finish().context("finish zip failed")?;
        Result::Ok(())
    }

    pub fn add_sound(&mut self) -> Result<(), Error> {
        for note in self.notes.borrow_mut().iter_mut() {
            for field in note.fields.iter_mut() {
                if field.starts_with("[sound:") {
                    *field = format!("[sound:{}.mp3]", note.sfld);
                }
            }
            let db = infra::repository::get_dbstorage().lock().unwrap();
            db.as_ref().unwrap().update_note(&note).unwrap();
            let p = PathBuf::from_str(&format!(
                "/Users/jasonjsyuan/Downloads/speech/{}/{}.mp3",
                &note.sfld.chars().nth(0).unwrap().to_string(),
                &note.sfld
            ))
            .context("get media path failed")?;
            let _ = self.add_media(Media {
                name: note.sfld.to_owned() + ".mp3",
                path: p,
            });
        }
        return Ok(());
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Media {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct CardTemplate {
    pub name: String,
    pub ord: i32,
    pub qfmt: String,
    pub afmt: String,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct NoteField {
    pub name: String,
    pub ord: i32,
    pub font: String,
    pub size: i32,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct NoteType {
    pub id: String,
    pub name: String,
    pub tmpls: Vec<CardTemplate>,
    pub flds: Vec<NoteField>,
}

impl NoteType {
    pub fn find_sound_field(&self) -> Option<&NoteField> {
        self.flds.iter().find(|x| x.name == "发音")
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct Conf {
    pub cur_model: NoteType,
}

impl Conf {
    pub fn new(cur_model: NoteType) -> Self {
        Self { cur_model }
    }

    pub fn get_cur_model(&self) -> &NoteType {
        return &self.cur_model;
    }
}
