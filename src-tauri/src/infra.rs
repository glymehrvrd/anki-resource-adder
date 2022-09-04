pub mod database {
    use serde_derive::Deserialize;
    use serde_derive::Serialize;
    use std::collections::HashMap;

    pub type Models = HashMap<String, Model>;

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Model {
        pub id: i64,
        pub name: String,
        pub tmpls: Vec<Tmpl>,
        pub flds: Vec<Fld>,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Tmpl {
        pub name: String,
        pub ord: i32,
        pub qfmt: String,
        pub afmt: String,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Fld {
        pub name: String,
        pub ord: i32,
        pub font: String,
        pub size: i32,
    }

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase")]
    pub struct Conf {
        pub cur_model: i64,
    }
}

pub mod storage {
    use std::path::Path;

    use super::database;
    use crate::domains;
    use easy_error::{bail, format_err, Error, ResultExt};

    pub trait Storage {
        fn get_cur_conf(&self) -> Result<domains::Conf, Error>;
        fn list_notes(&self) -> Result<Vec<domains::Note>, Error>;
        fn update_note(&self, note: &domains::Note) -> Result<(), Error>;
    }

    pub struct DBStorage {
        conn: rusqlite::Connection,
    }

    impl DBStorage {
        pub fn new(dbpath: &Path) -> Result<Self, Error> {
            let conn = rusqlite::Connection::open(dbpath).context("open failed")?;
            Ok(Self { conn })
        }
    }

    impl Storage for DBStorage {
        fn get_cur_conf(&self) -> Result<domains::Conf, Error> {
            let mut stmt = self
                .conn
                .prepare("select conf, models from col")
                .context("prepare statament failed")?;
            let mut rows = stmt.query([]).context(format!("query failed"))?;
            let conf_str: String;
            let model_str: String;
            (conf_str, model_str) = match rows.next().context("next failed")? {
                Some(row) => (
                    row.get(0).context("cannot get conf")?,
                    row.get(1).context("cannot get model")?,
                ),
                None => bail!(""),
            };
            let raw_conf: database::Conf =
                serde_json::from_str(&conf_str).context("parse conf failed")?;
            let raw_models: database::Models =
                serde_json::from_str(&model_str).context("parse model failed")?;
            let cur_model_id = raw_conf.cur_model.to_string();
            let raw_cur_model = raw_models.get(&cur_model_id).ok_or(format_err!(
                "cannot find curmodel|model_id={}|models={:#?}",
                cur_model_id,
                raw_models
            ))?;
            return Ok(domains::Conf {
                cur_model: domains::NoteType {
                    id: raw_cur_model.id.to_string(),
                    name: raw_cur_model.name.to_owned(),
                    tmpls: raw_cur_model
                        .tmpls
                        .iter()
                        .map(|val| domains::CardTemplate {
                            name: val.name.to_owned(),
                            ord: val.ord,
                            qfmt: val.qfmt.to_owned(),
                            afmt: val.afmt.to_owned(),
                        })
                        .collect(),
                    flds: raw_cur_model
                        .flds
                        .iter()
                        .map(|val| domains::NoteField {
                            name: val.name.to_owned(),
                            ord: val.ord,
                            font: val.font.to_owned(),
                            size: val.size,
                        })
                        .collect(),
                },
            });
        }

        fn list_notes(&self) -> Result<Vec<domains::Note>, Error> {
            let mut stmt = self
                .conn
                .prepare("select id, flds, sfld from notes")
                .context("prepare statament failed")?;
            return stmt
                .query_map([], |r| {
                    Ok(domains::Note {
                        id: r.get::<usize, i64>(0)?.to_string(),
                        fields: (r.get::<usize, String>(1)?)
                            .split("\x1f")
                            .map(|x| x.to_string())
                            .collect(),
                        sfld: r.get(2)?,
                    })
                })
                .context("query failed")?
                .map(|x| x.context("get index failed"))
                .collect();
        }

        fn update_note(&self, note: &domains::Note) -> Result<(), Error> {
            self.conn
                .execute(
                    "update notes set flds=? where id=?",
                    [&note.fields.join("\x1f"), &note.id],
                )
                .context("update note failed")?;
            return Ok(());
        }
    }
}

pub mod repository {
    use std::sync::Mutex;

    use super::storage::DBStorage;

    static STORAGE: Mutex<Option<DBStorage>> = Mutex::new(None);

    pub fn init_dbstorage(storage: DBStorage) {
        *STORAGE.lock().unwrap() = Some(storage);
    }

    pub fn get_dbstorage() -> &'static Mutex<Option<DBStorage>> {
        return &STORAGE;
    }
}
