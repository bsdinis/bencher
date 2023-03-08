use crate::*;

/// With this handle, it is possible to write to the set
pub struct LinearSetHandle<'a> {
    db: &'a DbWriteBackend,
    exp_code: String,
}

impl<'a> LinearSetHandle<'a> {
    pub(crate) fn new(db: &'a DbWriteBackend, exp_code: impl ToString) -> Self {
        LinearSetHandle {
            db,
            exp_code: exp_code.to_string(),
        }
    }

    /// Tag untagged datapoint (with the next point in set)
    fn tag_datapoint(&self, datapoint: LinearDatapoint) -> BencherResult<LinearDatapoint> {
        if let None = &datapoint.tag {
            let new_tag = self.db.get_new_linear_tag(&self.exp_code)?;

            Ok(datapoint.tag(new_tag))
        } else {
            Ok(datapoint)
        }
    }

    pub fn add_datapoint(&self, datapoint: LinearDatapoint) -> BencherResult<()> {
        let datapoint = self.tag_datapoint(datapoint)?;
        self.db.add_linear_datapoint(&self.exp_code, datapoint)
    }

    pub fn revert(&self, group: &str, version: Option<usize>) -> BencherResult<()> {
        self.db
            .revert_linear_datapoint(&self.exp_code, group, version)
    }
}

pub struct XYLineHandle<'a> {
    db: &'a DbWriteBackend,
    exp_code: String,
}

impl<'a> XYLineHandle<'a> {
    pub(crate) fn new(db: &'a DbWriteBackend, exp_code: impl ToString) -> Self {
        XYLineHandle {
            db,
            exp_code: exp_code.to_string(),
        }
    }

    /// Tag untagged datapoint (with the next point in line)
    fn tag_datapoint(&self, datapoint: XYDatapoint) -> BencherResult<XYDatapoint> {
        if let None = &datapoint.tag {
            let new_tag = self.db.get_new_xy_tag(&self.exp_code)?;

            Ok(datapoint.tag(new_tag))
        } else {
            Ok(datapoint)
        }
    }

    pub fn add_datapoint(&self, datapoint: XYDatapoint) -> BencherResult<()> {
        let datapoint = self.tag_datapoint(datapoint)?;
        self.db.add_xy_datapoint(&self.exp_code, datapoint)
    }

    pub fn revert(&self, tag: isize, version: Option<usize>) -> BencherResult<()> {
        self.db.revert_xy_datapoint(&self.exp_code, tag, version)
    }
}
