use std::collections::HashMap;
use std::net::Shutdown;
use std::net::TcpStream;

#[derive(Debug, Clone)]
pub struct MeaningDb {
    meanings: HashMap<String, String>,
}

impl MeaningDb {
    pub fn new(meanings: HashMap<String, String>) -> Self {
        Self { meanings }
    }

    pub fn meaning_for(&self, tag: &str) -> Option<&str> {
        self.meanings.get(tag).map(|s| s.as_str())
    }
}

#[derive(Debug, Clone)]
pub struct Verifier {
    meaning_db: MeaningDb,
}

impl Verifier {
    pub fn new(meaning_db: MeaningDb) -> Self {
        Self { meaning_db }
    }

    pub fn verify_or_disconnect(
        &self,
        tag: &str,
        payload: &str,
        stream: &TcpStream,
    ) -> bool {
        if let Some(required) = self.meaning_db.meaning_for(tag) {
            if !payload.contains(required) {
                let _ = stream.shutdown(Shutdown::Both);
                return false;
            }
        }
        true
    }
}
