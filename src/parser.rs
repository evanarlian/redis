use std::collections::HashMap;

pub struct OptionalArgs {
    pub args: HashMap<String, Option<String>>,
    pub flags: HashMap<String, bool>,
}
impl OptionalArgs {
    pub fn new(arg_keys: &[&str], flag_keys: &[&str]) -> Self {
        // assume there are no duplicates in args and flags
        Self {
            args: arg_keys
                .iter()
                .map(|k| (k.to_lowercase().to_string(), None))
                .collect(),
            flags: flag_keys
                .iter()
                .map(|k| (k.to_lowercase().to_string(), false))
                .collect(),
        }
    }
    pub fn insert_from_iter<T>(&mut self, it: &mut T) -> Result<(), String>
    where
        T: Iterator<Item = String>,
    {
        // try filling in from iterator, while erroring on offending key
        while let Some(x) = it.next() {
            let x = x.to_lowercase();
            if self.flags.contains_key(&x) {
                self.flags.insert(x, true);
            } else if self.args.contains_key(&x) {
                let value = it.next().ok_or(x.clone())?;
                self.args.insert(x, Some(value));
            } else {
                return Err(x);
            }
        }
        Ok(())
    }
}
// TODO refac: move to command module, SUBMODULE like resp