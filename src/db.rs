use rusqlite::OptionalExtension;
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::Path,
};

use either::Either;

use crate::*;

#[derive(Debug)]
pub(crate) struct DbWriteBackend {
    db: rusqlite::Connection,
}

impl From<DbWriteBackend> for rusqlite::Connection {
    fn from(value: DbWriteBackend) -> Self {
        value.db
    }
}

impl DbWriteBackend {
    pub(crate) fn new(path: &std::path::Path) -> BencherResult<Self> {
        let db = open_db(path)?;
        setup_db(&db)?;
        Ok(DbWriteBackend { db })
    }

    pub(crate) fn from_conn(conn: rusqlite::Connection) -> BencherResult<Self> {
        setup_db(&conn)?;
        Ok(DbWriteBackend { db: conn })
    }

    pub(crate) fn experiment_exists(
        &self,
        exp_type: &str,
        exp_label: &str,
        exp_code: &str,
    ) -> BencherResult<bool> {
        if let Some((existing_type, existing_label)) = self.db.query_row(
            "select experiment_type, experiment_label from experiments where experiment_code = :code",
            rusqlite::named_params! { ":code": exp_code },
            |row| Ok((row.get(0).unwrap_or("".into()), row.get(1).unwrap_or("".into())))
            ).optional()? {

            if &existing_type != exp_type {
                Err(BencherError::MismatchedType(exp_code.into(), existing_type, exp_type.into()))
            } else if &existing_label != exp_label {
                Err(BencherError::MismatchedLabel(exp_code.into(), existing_label, exp_label.into()))
            } else {
                Ok(true)
            }
        } else {
            Ok(false)
        }
    }

    pub(crate) fn code_exists(&self, exp_code: &str) -> BencherResult<bool> {
        if let Some(_) = self
            .db
            .query_row(
                "select * from experiments where experiment_code = :code",
                rusqlite::named_params! { ":code": exp_code },
                |_| Ok(Some(())),
            )
            .optional()?
        {
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub(crate) fn insert_linear_set(
        &self,
        exp_type: &str,
        exp_label: &str,
        exp_code: &str,
    ) -> BencherResult<()> {
        let mut stmt = self.db.prepare(
            "insert into experiments (
                    experiment_type,
                    experiment_label,
                    experiment_code
                    ) values (
                    :exp_type,
                    :exp_label,
                    :exp_code)",
        )?;

        stmt.execute(rusqlite::named_params! {
            ":exp_type": exp_type,
            ":exp_label": exp_label,
            ":exp_code": exp_code,
        })?;

        Ok(())
    }

    pub(crate) fn get_linear_set<'a>(
        &'a self,
        exp_code: &str,
    ) -> BencherResult<Option<LinearSetHandle<'a>>> {
        Ok(self
            .code_exists(exp_code)?
            .then_some(LinearSetHandle::new(self, exp_code)))
    }

    pub(crate) fn insert_xy_line(
        &self,
        exp_type: &str,
        exp_label: &str,
        exp_code: &str,
    ) -> BencherResult<()> {
        let mut stmt = self.db.prepare(
            "insert into experiments (
                    experiment_type,
                    experiment_label,
                    experiment_code
                    ) values (
                    :exp_type,
                    :exp_label,
                    :exp_code)",
        )?;

        stmt.execute(rusqlite::named_params! {
            ":exp_type": exp_type,
            ":exp_label": exp_label,
            ":exp_code": exp_code,
        })?;

        Ok(())
    }

    pub(crate) fn get_xy_line<'a>(
        &'a self,
        exp_code: &str,
    ) -> BencherResult<Option<XYLineHandle<'a>>> {
        Ok(self
            .code_exists(exp_code)?
            .then_some(XYLineHandle::new(self, exp_code)))
    }

    pub(crate) fn list_codes(&self) -> BencherResult<Vec<String>> {
        let mut stmt = self.db.prepare("select experiment_code from experiments")?;

        let result = stmt
            .query_map([], |row| Ok(row.get(0).unwrap_or("".to_string())))?
            .into_iter()
            .map(|x| x.map_err(|e| e.into()))
            .collect::<BencherResult<Vec<_>>>();

        result
    }

    // get the new version for a given datapoint
    fn get_new_linear_version(&self, exp_code: &str, group: &str) -> BencherResult<isize> {
        let new_version = self.db.query_row(
                "select max(abs(version)) + 1 from linear_results where experiment_code = :code and v_group = :v_group",
            rusqlite::named_params! { ":code": exp_code, ":v_group": group },
            |row| Ok(row.get(0).unwrap_or(1)),
        )?;

        Ok(new_version)
    }

    pub(crate) fn add_linear_datapoint(
        &self,
        exp_code: &str,
        datapoint: LinearDatapoint,
    ) -> BencherResult<()> {
        let version = self.get_new_linear_version(exp_code, &datapoint.group)?;
        let mut stmt = self.db.prepare(
            "insert into linear_results (
                    experiment_code,
                    version,
                    v_group,

                    v_int,
                    v_int_1,
                    v_int_5,
                    v_int_10,
                    v_int_25,
                    v_int_99,
                    v_int_95,
                    v_int_90,
                    v_int_75,

                    v_float,
                    v_float_1,
                    v_float_5,
                    v_float_10,
                    v_float_25,
                    v_float_99,
                    v_float_95,
                    v_float_90,
                    v_float_75
                ) values (
                    :experiment_code,
                    :version,
                    :v_group,

                    :v_int,
                    :v_int_1,
                    :v_int_5,
                    :v_int_10,
                    :v_int_25,
                    :v_int_99,
                    :v_int_95,
                    :v_int_90,
                    :v_int_75,

                    :v_float,
                    :v_float_1,
                    :v_float_5,
                    :v_float_10,
                    :v_float_25,
                    :v_float_99,
                    :v_float_95,
                    :v_float_90,
                    :v_float_75
                )",
        )?;

        stmt.execute(rusqlite::named_params! {
            ":experiment_code": exp_code,
            ":v_group": datapoint.group,
            ":version": version,

            ":v_int": datapoint.v.to_int(),
            ":v_float": datapoint.v.to_float(),

            ":v_int_1": datapoint.get_confidence(1).clone().map(|val| val.0.to_int()).flatten(),
            ":v_int_99": datapoint.get_confidence(1).clone().map(|val| val.1.to_int()).flatten(),

            ":v_int_5": datapoint.get_confidence(5).clone().map(|val| val.0.to_int()).flatten(),
            ":v_int_95": datapoint.get_confidence(5).clone().map(|val| val.1.to_int()).flatten(),

            ":v_int_10": datapoint.get_confidence(10).clone().map(|val| val.0.to_int()).flatten(),
            ":v_int_90": datapoint.get_confidence(10).clone().map(|val| val.1.to_int()).flatten(),

            ":v_int_25": datapoint.get_confidence(25).clone().map(|val| val.0.to_int()).flatten(),
            ":v_int_75": datapoint.get_confidence(25).clone().map(|val| val.1.to_int()).flatten(),

            ":v_float_1": datapoint.get_confidence(1).clone().map(|val| val.0.to_float()).flatten(),
            ":v_float_99": datapoint.get_confidence(1).clone().map(|val| val.1.to_float()).flatten(),

            ":v_float_5": datapoint.get_confidence(5).clone().map(|val| val.0.to_float()).flatten(),
            ":v_float_95": datapoint.get_confidence(5).clone().map(|val| val.1.to_float()).flatten(),

            ":v_float_10": datapoint.get_confidence(10).clone().map(|val| val.0.to_float()).flatten(),
            ":v_float_90": datapoint.get_confidence(10).clone().map(|val| val.1.to_float()).flatten(),

            ":v_float_25": datapoint.get_confidence(25).clone().map(|val| val.0.to_float()).flatten(),
            ":v_float_75": datapoint.get_confidence(25).clone().map(|val| val.1.to_float()).flatten(),
        })?;
        Ok(())
    }

    pub(crate) fn revert_linear_datapoint(
        &self,
        exp_code: &str,
        group: &str,
        version: Option<usize>,
    ) -> BencherResult<()> {
        if let Some(v) = version {
            self.db.execute("update linear_results set version = abs(version) where experiment_code = :code and v_group = :v_group and abs(version) = :version",
                            rusqlite::named_params! { ":code": exp_code, ":v_group": group, ":version": v})?;
            self.db.execute("update linear_results set version = -version where experiment_code = :code and v_group = :v_group and version > :version",
                            rusqlite::named_params! { ":code": exp_code, ":v_group": group, ":version": v})?;
        } else {
            self.db.execute("update linear_results set version = -version where experiment_code = :code and v_group = :v_group and version in
                            (select max(version) from linear_results where experiment_code = :code and v_group = :v_group)",
                            rusqlite::named_params! { ":code": exp_code, ":v_group": group })?;
        }
        Ok(())
    }

    fn get_new_xy_version(&self, exp_code: &str, tag: isize) -> BencherResult<isize> {
        self.db.query_row(
                "select max(abs(version)) + 1 from xy_results where experiment_code = :exp_code and tag = :tag",
            rusqlite::named_params! { ":exp_code": exp_code, ":tag": tag },
            |row| Ok(row.get(0).unwrap_or(1)),
        ).map_err(|e| e.into())
    }

    pub(crate) fn get_new_xy_tag(&self, exp_code: &str) -> BencherResult<isize> {
        self.db
            .query_row(
                "select max(tag) + 1 from xy_results where experiment_code = :code",
                rusqlite::named_params! { ":code": exp_code },
                |row| Ok(row.get(0).unwrap_or(0)),
            )
            .map_err(|e| e.into())
    }

    pub(crate) fn add_xy_datapoint(
        &self,
        exp_code: &str,
        datapoint: XYDatapoint,
    ) -> BencherResult<()> {
        let version = self.get_new_xy_version(exp_code, datapoint.tag.unwrap())?;
        let mut stmt = self.db.prepare(
            "insert into xy_results (
                    experiment_code,
                    tag,
                    version,

                    x_int,
                    x_int_1,
                    x_int_5,
                    x_int_10,
                    x_int_25,
                    x_int_99,
                    x_int_95,
                    x_int_90,
                    x_int_75,

                    y_int,
                    y_int_1,
                    y_int_5,
                    y_int_10,
                    y_int_25,
                    y_int_99,
                    y_int_95,
                    y_int_90,
                    y_int_75,

                    x_float,
                    x_float_1,
                    x_float_5,
                    x_float_10,
                    x_float_25,
                    x_float_99,
                    x_float_95,
                    x_float_90,
                    x_float_75,

                    y_float,
                    y_float_1,
                    y_float_5,
                    y_float_10,
                    y_float_25,
                    y_float_99,
                    y_float_95,
                    y_float_90,
                    y_float_75
                ) values (
                    :experiment_code,
                    :tag,
                    :version,

                    :x_int,
                    :x_int_1,
                    :x_int_5,
                    :x_int_10,
                    :x_int_25,
                    :x_int_99,
                    :x_int_95,
                    :x_int_90,
                    :x_int_75,

                    :y_int,
                    :y_int_1,
                    :y_int_5,
                    :y_int_10,
                    :y_int_25,
                    :y_int_99,
                    :y_int_95,
                    :y_int_90,
                    :y_int_75,

                    :x_float,
                    :x_float_1,
                    :x_float_5,
                    :x_float_10,
                    :x_float_25,
                    :x_float_99,
                    :x_float_95,
                    :x_float_90,
                    :x_float_75,

                    :y_float,
                    :y_float_1,
                    :y_float_5,
                    :y_float_10,
                    :y_float_25,
                    :y_float_99,
                    :y_float_95,
                    :y_float_90,
                    :y_float_75
                )",
        )?;

        stmt.execute(rusqlite::named_params! {
            ":experiment_code": exp_code,
            ":tag": datapoint.tag.unwrap(),
            ":version": version,

            ":x_int": datapoint.x.to_int(),
            ":x_float": datapoint.x.to_float(),
            ":y_int": datapoint.y.to_int(),
            ":y_float": datapoint.y.to_float(),

            ":x_int_1": datapoint.get_x_confidence(1).clone().map(|val| val.0.to_int()).flatten(),
            ":x_int_99": datapoint.get_x_confidence(1).clone().map(|val| val.1.to_int()).flatten(),

            ":x_int_5": datapoint.get_x_confidence(5).clone().map(|val| val.0.to_int()).flatten(),
            ":x_int_95": datapoint.get_x_confidence(5).clone().map(|val| val.1.to_int()).flatten(),

            ":x_int_10": datapoint.get_x_confidence(10).clone().map(|val| val.0.to_int()).flatten(),
            ":x_int_90": datapoint.get_x_confidence(10).clone().map(|val| val.1.to_int()).flatten(),

            ":x_int_25": datapoint.get_x_confidence(25).clone().map(|val| val.0.to_int()).flatten(),
            ":x_int_75": datapoint.get_x_confidence(25).clone().map(|val| val.1.to_int()).flatten(),

            ":x_float_1": datapoint.get_x_confidence(1).clone().map(|val| val.0.to_float()).flatten(),
            ":x_float_99": datapoint.get_x_confidence(1).clone().map(|val| val.1.to_float()).flatten(),

            ":x_float_5": datapoint.get_x_confidence(5).clone().map(|val| val.0.to_float()).flatten(),
            ":x_float_95": datapoint.get_x_confidence(5).clone().map(|val| val.1.to_float()).flatten(),

            ":x_float_10": datapoint.get_x_confidence(10).clone().map(|val| val.0.to_float()).flatten(),
            ":x_float_90": datapoint.get_x_confidence(10).clone().map(|val| val.1.to_float()).flatten(),

            ":x_float_25": datapoint.get_x_confidence(25).clone().map(|val| val.0.to_float()).flatten(),
            ":x_float_75": datapoint.get_x_confidence(25).clone().map(|val| val.1.to_float()).flatten(),

            ":y_int_1": datapoint.get_y_confidence(1).clone().map(|val| val.0.to_int()).flatten(),
            ":y_int_99": datapoint.get_y_confidence(1).clone().map(|val| val.1.to_int()).flatten(),

            ":y_int_5": datapoint.get_y_confidence(5).clone().map(|val| val.0.to_int()).flatten(),
            ":y_int_95": datapoint.get_y_confidence(5).clone().map(|val| val.1.to_int()).flatten(),

            ":y_int_10": datapoint.get_y_confidence(10).clone().map(|val| val.0.to_int()).flatten(),
            ":y_int_90": datapoint.get_y_confidence(10).clone().map(|val| val.1.to_int()).flatten(),

            ":y_int_25": datapoint.get_y_confidence(25).clone().map(|val| val.0.to_int()).flatten(),
            ":y_int_75": datapoint.get_y_confidence(25).clone().map(|val| val.1.to_int()).flatten(),

            ":y_float_1": datapoint.get_y_confidence(1).clone().map(|val| val.0.to_float()).flatten(),
            ":y_float_99": datapoint.get_y_confidence(1).clone().map(|val| val.1.to_float()).flatten(),

            ":y_float_5": datapoint.get_y_confidence(5).clone().map(|val| val.0.to_float()).flatten(),
            ":y_float_95": datapoint.get_y_confidence(5).clone().map(|val| val.1.to_float()).flatten(),

            ":y_float_10": datapoint.get_y_confidence(10).clone().map(|val| val.0.to_float()).flatten(),
            ":y_float_90": datapoint.get_y_confidence(10).clone().map(|val| val.1.to_float()).flatten(),

            ":y_float_25": datapoint.get_y_confidence(25).clone().map(|val| val.0.to_float()).flatten(),
            ":y_float_75": datapoint.get_y_confidence(25).clone().map(|val| val.1.to_float()).flatten(),
        })?;
        Ok(())
    }

    pub(crate) fn revert_xy_datapoint(
        &self,
        exp_code: &str,
        tag: isize,
        version: Option<usize>,
    ) -> BencherResult<()> {
        if let Some(v) = version {
            self.db.execute("update xy_results set version = abs(version) where experiment_code = :code and tag = :tag and abs(version) = :version",
                            rusqlite::named_params! { ":code": exp_code, ":tag": tag, ":version": v})?;
            self.db.execute("update xy_results set version = -version where experiment_code = :code and tag = :tag and version > :version",
                            rusqlite::named_params! { ":code": exp_code, ":tag": tag, ":version": v})?;
        } else {
            self.db.execute("update xy_results set version = -version where experiment_code = :code and tag = :tag and version in
                            (select max(version) from xy_results where experiment_code = :code and tag = :tag)",
                            rusqlite::named_params! { ":code": exp_code, ":tag": tag })?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct DbReadBackend {
    dbs: Vec<rusqlite::Connection>,

    /// map from experiment code to index of the DB that has this code
    ///
    /// Building this allows us to have lookups to the DBs used
    code_map: HashMap<String, usize>,
}

impl DbReadBackend {
    fn build_code_map(dbs: &Vec<rusqlite::Connection>) -> BencherResult<HashMap<String, usize>> {
        let (inverted_map, codesets) = {
            let mut map = HashMap::new();
            let mut codesets = Vec::new();

            for (idx, conn) in dbs.iter().enumerate() {
                let mut codeset = HashSet::new();
                let mut stmt = conn.prepare("select experiment_code from experiments")?;

                for code in stmt.query_map([], |row| Ok(row.get(0).unwrap_or("".to_string())))? {
                    let code = code.unwrap();
                    map.insert(code.clone(), idx);
                    codeset.insert(code);
                }

                codesets.push(codeset);
            }

            Ok::<(HashMap<_, _>, Vec<_>), BencherError>((map, codesets))
        }?;

        let mut all_codes = HashSet::new();

        // check that all code sets are disjoint
        //
        // all sets are piecewise disjoint iff
        //  \forall i, set #i is disjoint with the union of sets #0 through #i-1
        for (idx, codeset) in codesets.into_iter().enumerate() {
            if all_codes.is_disjoint(&codeset) {
                all_codes = all_codes.union(&codeset).cloned().collect::<HashSet<_>>()
            } else {
                return Err(BencherError::IncompatibleDbs {
                    db: dbs[idx].path().unwrap().into(),
                    codes: all_codes
                        .intersection(&codeset)
                        .cloned()
                        .collect::<HashSet<_>>(),
                });
            }
        }

        Ok(inverted_map)
    }

    pub(crate) fn from_conns(dbs: Vec<rusqlite::Connection>) -> BencherResult<Self> {
        for db in dbs.iter() {
            check_compatible_db(db)?;
        }
        let code_map = Self::build_code_map(&dbs)?;
        Ok(DbReadBackend { dbs, code_map })
    }

    pub(crate) fn new<'a>(
        default_path: &std::path::Path,
        paths: impl Iterator<Item = &'a std::path::Path>,
    ) -> BencherResult<Self> {
        let default_db = open_db(default_path)?;
        let mut dbs = open_dbs(paths)?;
        dbs.push(default_db);
        Self::from_conns(dbs)
    }

    pub(crate) fn from_paths<'a>(
        paths: impl Iterator<Item = &'a std::path::Path>,
    ) -> BencherResult<Self> {
        let dbs = open_dbs(paths)?;
        Self::from_conns(dbs)
    }

    pub(crate) fn get_linear_datapoints(&self, code: &str) -> BencherResult<Vec<LinearDatapoint>> {
        let mut vec = vec![];

        let mut stmt = self.dbs[self.code_map[code]].prepare(
            "select v_group, v_int, v_float,
                    v_int_1,    v_int_99,
                    v_float_1,  v_float_99,

                    v_int_5,    v_int_95,
                    v_float_5,  v_float_95,

                    v_int_10,   v_int_90,
                    v_float_10, v_float_90,

                    v_int_25,   v_int_75,
                    v_float_25, v_float_75,

                    max(version)
             from linear_results
             where experiment_code = :code
             group by v_group
             ",
        )?;

        for datapoint in stmt.query_map(rusqlite::named_params! { ":code": code }, |row| {
            LinearDatapoint::try_from(row).map_err(|e| e.into())
        })? {
            vec.push(datapoint?);
        }

        vec.sort_by_key(|d| d.group.clone());
        Ok(vec)
    }

    pub(crate) fn get_xy_datapoints(&self, code: &str) -> BencherResult<Vec<XYDatapoint>> {
        let mut vec = vec![];

        let mut stmt = self.dbs[self.code_map[code]].prepare(
            "select x_int, x_float,
                y_int, y_float,
                x_int_1,    x_int_99,
                x_float_1,  x_float_99,

                x_int_5,    x_int_95,
                x_float_5,  x_float_95,

                x_int_10,   x_int_90,
                x_float_10, x_float_90,

                x_int_25,   x_int_75,
                x_float_25, x_float_75,

                y_int_1,    y_int_99,
                y_float_1,  y_float_99,

                y_int_5,    y_int_95,
                y_float_5,  y_float_95,

                y_int_10,   y_int_90,
                y_float_10, y_float_90,

                y_int_25,   y_int_75,
                y_float_25, y_float_75,

                tag, max(version)
         from xy_results
         where experiment_code = :code
         group by tag
         ",
        )?;

        for datapoint in stmt.query_map(rusqlite::named_params! { ":code": code }, |row| {
            XYDatapoint::try_from(row).map_err(|e| e.into())
        })? {
            vec.push(datapoint?);
        }

        vec.sort_by_key(|d| d.tag);
        Ok(vec)
    }

    pub(crate) fn status(&self) -> BencherResult<Vec<ExperimentStatus>> {
        let mut map = BTreeMap::new();

        for db in &self.dbs {
            let mut stmt = db.prepare(
                "select experiment_code, experiment_label, experiment_type from experiments",
            )?;
            for status in stmt.query_map([], |row| {
                Ok(ExperimentStatus {
                    database: db
                        .path()
                        .map(|d| d.to_str().unwrap_or("<unknown>"))
                        .unwrap_or("<unknown")
                        .to_string(),
                    exp_type: row.get(2).unwrap_or("".to_string()),
                    exp_label: row.get(1).unwrap_or("".to_string()),
                    exp_code: row.get(0).unwrap_or("".to_string()),
                    n_datapoints: 0,
                    n_active_datapoints: 0,
                })
            })? {
                let status = status.unwrap();
                map.insert(status.exp_code.clone(), status);
            }

            let mut stmt = db
                .prepare("select experiment_code, count(*) from xy_results union select experiment_code, count(*) from linear_results group by experiment_code")?;
            for status in stmt.query_map([], |row| {
                Ok((
                    row.get(0).unwrap_or("".to_string()),
                    row.get(1).unwrap_or(0),
                ))
            })? {
                let (code, n_datapoints) = status.unwrap();
                map.get_mut(&code).map(|s| s.n_datapoints = n_datapoints);
            }

            let mut stmt =
                db.prepare("select experiment_code, tag, max(version) from xy_results")?;
            for code in stmt.query_map([], |row| Ok(row.get(0).unwrap_or("".to_string())))? {
                map.get_mut(&code.unwrap())
                    .map(|s| s.n_active_datapoints += 1);
            }

            let mut stmt = db
                .prepare("select experiment_code, v_group, max(version) from linear_results group by experiment_code, v_group")?;
            for code in stmt.query_map([], |row| Ok(row.get(0).unwrap_or("".to_string())))? {
                map.get_mut(&code.unwrap())
                    .map(|s| s.n_active_datapoints += 1);
            }
        }

        let mut vector: Vec<_> = map.into_iter().map(|(_, v)| v).collect();
        // rust sort is stable

        vector.sort_by(|a, b| a.exp_code.cmp(&b.exp_code));
        vector.sort_by(|a, b| a.database.cmp(&b.database));
        Ok(vector)
    }

    pub(crate) fn list_linear_experiments(
        &self,
        linear_experiments: &Vec<LinearExperiment>,
    ) -> BencherResult<Vec<LinearExperimentInfo>> {
        let mut list = Vec::new();

        for db in &self.dbs {
            let database = db
                .path()
                .map(|d| d.to_str().unwrap_or("<unknown>"))
                .unwrap_or("<unknown")
                .to_string();
            let mut stmt = db.prepare(
                "select experiment_code, experiment_label, experiment_type from experiments join linear_results on experiments.experiment_code = linear_results.experiment_code",
            )?;
            for info in stmt.query_map([], |row| {
                let exp_type = row.get(2).unwrap_or("".to_string());
                if let Some(linear_experiment) =
                    linear_experiments.iter().find(|e| e.exp_type == exp_type)
                {
                    Ok(Some(LinearExperimentInfo {
                        database: database.clone(),
                        exp_code: row.get(0).unwrap_or("".to_string()),
                        exp_label: row.get(1).unwrap_or("".to_string()),
                        exp_type,
                        horizontal_label: linear_experiment.horizontal_label.clone(),
                        v_label: linear_experiment.v_label.clone(),
                        v_units: linear_experiment.v_units.clone(),
                    }))
                } else {
                    Ok(None)
                }
            })? {
                if let Some(info) = info.unwrap() {
                    list.push(info);
                }
            }
        }

        list.sort_by(|a, b| a.exp_code.cmp(&b.exp_code));
        list.sort_by(|a, b| a.database.cmp(&b.database));
        Ok(list)
    }

    pub(crate) fn list_xy_experiments(
        &self,
        xy_experiments: &Vec<XYExperiment>,
    ) -> BencherResult<Vec<XYExperimentInfo>> {
        let mut list = Vec::new();

        for db in &self.dbs {
            let database = db
                .path()
                .map(|d| d.to_str().unwrap_or("<unknown>"))
                .unwrap_or("<unknown")
                .to_string();
            let mut stmt = db.prepare(
                "select experiment_code, experiment_label, experiment_type from experiments join xy_results on experiments.experiment_code = xy_results.experiment_code",
            )?;
            for info in stmt.query_map([], |row| {
                let exp_type = row.get(2).unwrap_or("".to_string());
                if let Some(xy_experiment) = xy_experiments.iter().find(|e| e.exp_type == exp_type)
                {
                    Ok(Some(XYExperimentInfo {
                        database: database.clone(),
                        exp_code: row.get(0).unwrap_or("".to_string()),
                        exp_label: row.get(1).unwrap_or("".to_string()),
                        exp_type,
                        x_label: xy_experiment.x_label.clone(),
                        x_units: xy_experiment.x_units.clone(),
                        y_label: xy_experiment.y_label.clone(),
                        y_units: xy_experiment.y_units.clone(),
                    }))
                } else {
                    Ok(None)
                }
            })? {
                if let Some(info) = info.unwrap() {
                    list.push(info);
                }
            }
        }

        list.sort_by(|a, b| a.exp_code.cmp(&b.exp_code));
        list.sort_by(|a, b| a.database.cmp(&b.database));
        Ok(list)
    }

    pub(crate) fn list_codes(&self) -> BencherResult<Vec<String>> {
        let mut vec = vec![];
        for db in &self.dbs {
            let mut stmt = db.prepare("select experiment_code from experiments")?;

            let mut inner = stmt
                .query_map([], |row| Ok(row.get(0).unwrap_or("".to_string())))?
                .into_iter()
                .map(|x| x.map_err(|e| e.into()))
                .collect::<BencherResult<Vec<_>>>()?;
            vec.append(&mut inner);
        }

        Ok(vec)
    }

    pub(crate) fn list_codes_labels_by_exp_type(
        &self,
        exp_type: &str,
    ) -> BencherResult<Vec<(String, String)>> {
        let mut vec = vec![];
        for db in &self.dbs {
            let mut stmt = db.prepare(
                "select experiment_code, experiment_label from experiments where experiment_type=:exp_type",
            )?;

            let mut inner = stmt
                .query_map(rusqlite::named_params! { ":exp_type": exp_type}, |row| {
                    Ok((
                        row.get(0).unwrap_or("".to_string()),
                        row.get(1).unwrap_or("".to_string()),
                    ))
                })?
                .into_iter()
                .map(|x| x.map_err(|e| e.into()))
                .collect::<BencherResult<Vec<_>>>()?;
            vec.append(&mut inner);
        }

        Ok(vec)
    }
}

fn create_confidence_arg(
    min_int: Option<i64>,
    max_int: Option<i64>,
    min_float: Option<f64>,
    max_float: Option<f64>,
) -> Option<Either<(i64, i64), (f64, f64)>> {
    if let (Some(lower), Some(upper)) = (min_int, max_int) {
        Some(Either::Left((lower, upper)))
    } else if let (Some(lower), Some(upper)) = (min_float, max_float) {
        Some(Either::Right((lower, upper)))
    } else {
        None
    }
}

impl TryFrom<&rusqlite::Row<'_>> for LinearDatapoint {
    type Error = BencherError;
    fn try_from(row: &rusqlite::Row) -> BencherResult<Self> {
        let mut datapoint = LinearDatapoint::new(
            row.get::<usize, String>(0).unwrap(),
            Value::new(row.get(1).unwrap(), row.get(2).unwrap())?,
        );

        // x 1 - 99
        if let Some(e) = create_confidence_arg(
            row.get(3).unwrap(),
            row.get(4).unwrap(),
            row.get(5).unwrap(),
            row.get(6).unwrap(),
        ) {
            let _ = datapoint.add_confidence(1, e);
        }

        // x 5 - 95
        if let Some(e) = create_confidence_arg(
            row.get(7).unwrap(),
            row.get(8).unwrap(),
            row.get(9).unwrap(),
            row.get(10).unwrap(),
        ) {
            let _ = datapoint.add_confidence(5, e);
        }

        // x 10 - 90
        if let Some(e) = create_confidence_arg(
            row.get(11).unwrap(),
            row.get(12).unwrap(),
            row.get(13).unwrap(),
            row.get(14).unwrap(),
        ) {
            let _ = datapoint.add_confidence(10, e);
        }

        // x 25 - 75
        if let Some(e) = create_confidence_arg(
            row.get(15).unwrap(),
            row.get(16).unwrap(),
            row.get(17).unwrap(),
            row.get(18).unwrap(),
        ) {
            let _ = datapoint.add_confidence(10, e);
        }

        Ok(datapoint)
    }
}

impl TryFrom<&rusqlite::Row<'_>> for XYDatapoint {
    type Error = BencherError;
    fn try_from(row: &rusqlite::Row) -> BencherResult<Self> {
        let mut datapoint = XYDatapoint::new(
            Value::new(row.get(0).unwrap(), row.get(1).unwrap())?,
            Value::new(row.get(2).unwrap(), row.get(3).unwrap())?,
        );

        // x 1 - 99
        if let Some(e) = create_confidence_arg(
            row.get(4).unwrap(),
            row.get(5).unwrap(),
            row.get(6).unwrap(),
            row.get(7).unwrap(),
        ) {
            let _ = datapoint.add_x_confidence(1, e);
        }

        // x 5 - 95
        if let Some(e) = create_confidence_arg(
            row.get(8).unwrap(),
            row.get(9).unwrap(),
            row.get(10).unwrap(),
            row.get(11).unwrap(),
        ) {
            let _ = datapoint.add_x_confidence(5, e);
        }

        // x 10 - 90
        if let Some(e) = create_confidence_arg(
            row.get(12).unwrap(),
            row.get(13).unwrap(),
            row.get(14).unwrap(),
            row.get(15).unwrap(),
        ) {
            let _ = datapoint.add_x_confidence(10, e);
        }

        // x 25 - 75
        if let Some(e) = create_confidence_arg(
            row.get(16).unwrap(),
            row.get(17).unwrap(),
            row.get(18).unwrap(),
            row.get(19).unwrap(),
        ) {
            let _ = datapoint.add_x_confidence(10, e);
        }

        // y 1 - 99
        if let Some(e) = create_confidence_arg(
            row.get(20).unwrap(),
            row.get(21).unwrap(),
            row.get(22).unwrap(),
            row.get(23).unwrap(),
        ) {
            let _ = datapoint.add_y_confidence(1, e);
        }

        // y 5 - 95
        if let Some(e) = create_confidence_arg(
            row.get(24).unwrap(),
            row.get(25).unwrap(),
            row.get(26).unwrap(),
            row.get(27).unwrap(),
        ) {
            let _ = datapoint.add_y_confidence(5, e);
        }

        // y 10 - 90
        if let Some(e) = create_confidence_arg(
            row.get(28).unwrap(),
            row.get(29).unwrap(),
            row.get(30).unwrap(),
            row.get(31).unwrap(),
        ) {
            let _ = datapoint.add_y_confidence(10, e);
        }

        // y 25 - 75
        if let Some(e) = create_confidence_arg(
            row.get(32).unwrap(),
            row.get(33).unwrap(),
            row.get(34).unwrap(),
            row.get(35).unwrap(),
        ) {
            let _ = datapoint.add_y_confidence(10, e);
        }

        Ok(if let Some(tag) = row.get(36).unwrap() {
            datapoint.tag(tag)
        } else {
            datapoint
        })
    }
}

fn open_dbs<'a>(
    paths: impl Iterator<Item = &'a std::path::Path>,
) -> BencherResult<Vec<rusqlite::Connection>> {
    paths.map(open_db).collect::<BencherResult<Vec<_>>>()
}

fn open_db(db_path: &Path) -> BencherResult<rusqlite::Connection> {
    let flags = rusqlite::OpenFlags::SQLITE_OPEN_READ_WRITE
        | rusqlite::OpenFlags::SQLITE_OPEN_FULL_MUTEX
        | rusqlite::OpenFlags::SQLITE_OPEN_CREATE;

    let conn = rusqlite::Connection::open_with_flags(db_path, flags)
        .map_err(|e| BencherError::Database(e))?;
    setup_db(&conn)?;
    Ok(conn)
}

/// Check
fn check_compatible_db(db: &rusqlite::Connection) -> BencherResult<()> {
    fn table_exists(db: &rusqlite::Connection, name: &str) -> BencherResult<bool> {
        Ok(db
            .query_row(
                "select name from sqlite_schema where type=:type and name=:name",
                rusqlite::named_params! { ":type": "table", ":name": name },
                |_| Ok(()),
            )
            .optional()?
            .is_some())
    }

    if !table_exists(db, "experiments")? {
        return Err(BencherError::SchemaMissingTable(
            "experiments".to_string(),
            db.path()
                .map(|p| p.to_str().unwrap_or("<unrepresentable db name>"))
                .unwrap_or("<unknown db name>")
                .to_owned(),
        ));
    }

    if !table_exists(db, "linear_results")? {
        return Err(BencherError::SchemaMissingTable(
            "linear_results".to_string(),
            db.path()
                .map(|p| p.to_str().unwrap_or("<unrepresentable db name>"))
                .unwrap_or("<unknown db name>")
                .to_owned(),
        ));
    }

    if !table_exists(db, "xy_results")? {
        return Err(BencherError::SchemaMissingTable(
            "xy_results".to_string(),
            db.path()
                .map(|p| p.to_str().unwrap_or("<unrepresentable db name>"))
                .unwrap_or("<unknown db name>")
                .to_owned(),
        ));
    }

    Ok(())
}

fn setup_db(db: &rusqlite::Connection) -> BencherResult<()> {
    db.execute(
        "create table if not exists experiments (
            experiment_code text not null primary key,
            experiment_type text not null,
            experiment_label text not null
        )",
        [],
    )?;

    db.execute(
        "create table if not exists xy_results (
            experiment_code text not null,
            tag int not null,
            version int not null,

            x_int int,
            x_int_1 int,
            x_int_5 int,
            x_int_10 int,
            x_int_25 int,
            x_int_99 int,
            x_int_95 int,
            x_int_90 int,
            x_int_75 int,

            y_int int,
            y_int_1 int,
            y_int_5 int,
            y_int_10 int,
            y_int_25 int,
            y_int_99 int,
            y_int_95 int,
            y_int_90 int,
            y_int_75 int,

            x_float float,
            x_float_1 float,
            x_float_5 float,
            x_float_10 float,
            x_float_25 float,
            x_float_99 float,
            x_float_95 float,
            x_float_90 float,
            x_float_75 float,

            y_float float,
            y_float_1 float,
            y_float_5 float,
            y_float_10 float,
            y_float_25 float,
            y_float_99 float,
            y_float_95 float,
            y_float_90 float,
            y_float_75 float,

            foreing key experiment_code references experiments,
            primary key (experiment_code, tag, version)
        )",
        [],
    )?;

    db.execute(
        "create table if not exists linear_results (
            experiment_code text not null,
            v_group text not null,
            version int not null,

            v_int int,
            v_int_1 int,
            v_int_5 int,
            v_int_10 int,
            v_int_25 int,
            v_int_99 int,
            v_int_95 int,
            v_int_90 int,
            v_int_75 int,

            v_float float,
            v_float_1 float,
            v_float_5 float,
            v_float_10 float,
            v_float_25 float,
            v_float_99 float,
            v_float_95 float,
            v_float_90 float,
            v_float_75 float,

            foreing key experiment_code references experiments,
            primary key (experiment_code, v_group, version)
        )",
        [],
    )?;
    Ok(())
}
