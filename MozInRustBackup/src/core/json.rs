//! Analyseur JSON minimaliste (lecture seule), sans dépendance externe.
//!
//! Suffisant pour `profiles.json` et pour d'éventuels fichiers de
//! configuration ; ne sert jamais à réécrire du JSON.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<Value>),
    Object(BTreeMap<String, Value>),
}

impl Value {
    pub fn get(&self, key: &str) -> Option<&Value> {
        match self {
            Value::Object(m) => m.get(key),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s.as_str()),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&Vec<Value>> {
        match self {
            Value::Array(a) => Some(a),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct JsonError {
    pub message: String,
    pub position: usize,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "JSON invalide à la position {} : {}", self.position, self.message)
    }
}

impl std::error::Error for JsonError {}

pub fn parse(text: &str) -> Result<Value, JsonError> {
    let mut p = Parser { bytes: text.as_bytes(), pos: 0 };
    p.skip_ws();
    let v = p.value()?;
    Ok(v)
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn skip_ws(&mut self) {
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b' ' | b'\t' | b'\n' | b'\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn err(&self, msg: &str) -> JsonError {
        JsonError { message: msg.to_string(), position: self.pos }
    }

    fn value(&mut self) -> Result<Value, JsonError> {
        self.skip_ws();
        if self.pos >= self.bytes.len() {
            return Err(self.err("fin de fichier inattendue"));
        }
        match self.bytes[self.pos] {
            b'{' => self.object(),
            b'[' => self.array(),
            b'"' => Ok(Value::String(self.string()?)),
            b't' => self.literal("true", Value::Bool(true)),
            b'f' => self.literal("false", Value::Bool(false)),
            b'n' => self.literal("null", Value::Null),
            _ => self.number(),
        }
    }

    fn literal(&mut self, word: &str, value: Value) -> Result<Value, JsonError> {
        if self.bytes[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(value)
        } else {
            Err(self.err("littéral inconnu"))
        }
    }

    fn object(&mut self) -> Result<Value, JsonError> {
        let mut map = BTreeMap::new();
        self.pos += 1; // {
        loop {
            self.skip_ws();
            if self.pos >= self.bytes.len() {
                return Err(self.err("objet non terminé"));
            }
            if self.bytes[self.pos] == b'}' {
                self.pos += 1;
                break;
            }
            let key = self.string()?;
            self.skip_ws();
            if self.pos >= self.bytes.len() || self.bytes[self.pos] != b':' {
                return Err(self.err("« : » attendu"));
            }
            self.pos += 1;
            let v = self.value()?;
            map.insert(key, v);
            self.skip_ws();
            if self.pos < self.bytes.len() && self.bytes[self.pos] == b',' {
                self.pos += 1;
                continue;
            }
            if self.pos < self.bytes.len() && self.bytes[self.pos] == b'}' {
                self.pos += 1;
                break;
            }
            return Err(self.err("« , » ou « } » attendu"));
        }
        Ok(Value::Object(map))
    }

    fn array(&mut self) -> Result<Value, JsonError> {
        let mut items = Vec::new();
        self.pos += 1; // [
        loop {
            self.skip_ws();
            if self.pos >= self.bytes.len() {
                return Err(self.err("tableau non terminé"));
            }
            if self.bytes[self.pos] == b']' {
                self.pos += 1;
                break;
            }
            items.push(self.value()?);
            self.skip_ws();
            if self.pos < self.bytes.len() && self.bytes[self.pos] == b',' {
                self.pos += 1;
                continue;
            }
            if self.pos < self.bytes.len() && self.bytes[self.pos] == b']' {
                self.pos += 1;
                break;
            }
            return Err(self.err("« , » ou « ] » attendu"));
        }
        Ok(Value::Array(items))
    }

    fn string(&mut self) -> Result<String, JsonError> {
        self.skip_ws();
        if self.pos >= self.bytes.len() || self.bytes[self.pos] != b'"' {
            return Err(self.err("chaîne attendue"));
        }
        self.pos += 1;
        let mut out = String::new();
        while self.pos < self.bytes.len() {
            let c = self.bytes[self.pos];
            match c {
                b'"' => {
                    self.pos += 1;
                    return Ok(out);
                }
                b'\\' => {
                    self.pos += 1;
                    if self.pos >= self.bytes.len() {
                        return Err(self.err("échappement incomplet"));
                    }
                    let e = self.bytes[self.pos];
                    self.pos += 1;
                    match e {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'/' => out.push('/'),
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'u' => {
                            if self.pos + 4 > self.bytes.len() {
                                return Err(self.err("échappement unicode incomplet"));
                            }
                            let hex = std::str::from_utf8(&self.bytes[self.pos..self.pos + 4])
                                .map_err(|_| self.err("unicode invalide"))?;
                            let code = u32::from_str_radix(hex, 16)
                                .map_err(|_| self.err("unicode invalide"))?;
                            self.pos += 4;
                            out.push(char::from_u32(code).unwrap_or('\u{fffd}'));
                        }
                        _ => return Err(self.err("échappement inconnu")),
                    }
                }
                _ => {
                    // Copie du caractère UTF-8 complet.
                    let start = self.pos;
                    let len = utf8_len(c);
                    if start + len > self.bytes.len() {
                        return Err(self.err("UTF-8 tronqué"));
                    }
                    let s = std::str::from_utf8(&self.bytes[start..start + len])
                        .map_err(|_| self.err("UTF-8 invalide"))?;
                    out.push_str(s);
                    self.pos += len;
                }
            }
        }
        Err(self.err("chaîne non terminée"))
    }

    fn number(&mut self) -> Result<Value, JsonError> {
        let start = self.pos;
        while self.pos < self.bytes.len() {
            match self.bytes[self.pos] {
                b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9' => self.pos += 1,
                _ => break,
            }
        }
        if start == self.pos {
            return Err(self.err("valeur numérique attendue"));
        }
        let s = std::str::from_utf8(&self.bytes[start..self.pos]).unwrap_or("");
        s.parse::<f64>().map(Value::Number).map_err(|_| self.err("nombre invalide"))
    }
}

fn utf8_len(first: u8) -> usize {
    if first < 0x80 {
        1
    } else if first >> 5 == 0b110 {
        2
    } else if first >> 4 == 0b1110 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_object() {
        let v = parse(r#"{"a": 1, "b": "x", "c": [true, false], "d": {"e": null}}"#).unwrap();
        assert_eq!(v.get("a"), Some(&Value::Number(1.0)));
        assert_eq!(v.get("b").and_then(|v| v.as_str()), Some("x"));
        assert!(v.get("c").and_then(|v| v.as_array()).is_some());
        assert_eq!(v.get("d").and_then(|d| d.get("e")), Some(&Value::Null));
    }

    #[test]
    fn escapes() {
        let v = parse(r#"{"chemin": "C:\\Users\\moi", "accents":"éàü"}"#).unwrap();
        assert_eq!(v.get("chemin").and_then(|v| v.as_str()), Some("C:\\Users\\moi"));
        assert_eq!(v.get("accents").and_then(|v| v.as_str()), Some("éàü"));
    }

    #[test]
    fn errors() {
        assert!(parse("{").is_err());
        assert!(parse("").is_err());
    }
}
