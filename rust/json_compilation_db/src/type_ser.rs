use serde::ser::{Serialize, Serializer, SerializeStruct};

use crate::Entry;

impl Serialize for Entry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
    {
        let size = if self.output.is_some() { 4 } else { 3 };
        let mut state = serializer.serialize_struct("Entry", size)?;
        state.serialize_field("directory", &self.directory)?;
        state.serialize_field("file", &self.file)?;
        state.serialize_field("arguments", &self.arguments)?;
        if self.output.is_some() {
            state.serialize_field("output", &self.output)?;
        }
        state.end()
    }
}
