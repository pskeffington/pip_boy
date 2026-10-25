//! Fallout 4 companion wire decoder, independently implemented from published specs.
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::io::{self, Cursor, Read};
pub const MAX_FRAME: usize = 8 * 1024 * 1024;
fn bad() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidData,
        "invalid or excessive Fallout 4 data",
    )
}
fn bytes<const N: usize>(r: &mut impl Read) -> io::Result<[u8; N]> {
    let mut b = [0; N];
    r.read_exact(&mut b)?;
    Ok(b)
}
fn u32le(r: &mut impl Read) -> io::Result<u32> {
    Ok(u32::from_le_bytes(bytes(r)?))
}
fn count(r: &mut impl Read) -> io::Result<usize> {
    Ok(u16::from_le_bytes(bytes(r)?) as usize)
}
fn string(r: &mut impl Read) -> io::Result<String> {
    let mut b = Vec::new();
    loop {
        let c = bytes::<1>(r)?[0];
        if c == 0 {
            break;
        }
        b.push(c);
        if b.len() > 4096 {
            return Err(bad());
        }
    }
    Ok(String::from_utf8_lossy(&b).into_owned())
}
#[derive(Clone, Serialize)]
enum Node {
    Scalar(Value),
    Array(Vec<u32>),
    Object(BTreeMap<String, u32>),
}
#[derive(Default)]
pub struct Database {
    nodes: BTreeMap<u32, Node>,
}
impl Database {
    /// Commit only a complete valid update. Removals precede additions.
    pub fn update(&mut self, payload: &[u8]) -> io::Result<()> {
        if payload.len() > MAX_FRAME {
            return Err(bad());
        }
        let mut next = self.nodes.clone();
        let mut r = Cursor::new(payload);
        while (r.position() as usize) < payload.len() {
            let tag = bytes::<1>(&mut r)?[0];
            let id = u32le(&mut r)?;
            let node = match tag {
                0 => Node::Scalar(json!(bytes::<1>(&mut r)?[0] != 0)),
                1 => Node::Scalar(json!(bytes::<1>(&mut r)?[0] as i8)),
                2 => Node::Scalar(json!(bytes::<1>(&mut r)?[0])),
                3 => Node::Scalar(json!(i32::from_le_bytes(bytes(&mut r)?))),
                4 => Node::Scalar(json!(u32le(&mut r)?)),
                5 => {
                    let f = f32::from_le_bytes(bytes(&mut r)?);
                    if !f.is_finite() {
                        return Err(bad());
                    }
                    Node::Scalar(json!(f))
                }
                6 => Node::Scalar(json!(string(&mut r)?)),
                7 => {
                    let n = count(&mut r)?;
                    let a = (0..n)
                        .map(|_| u32le(&mut r))
                        .collect::<io::Result<Vec<_>>>()?;
                    Node::Array(a)
                }
                8 => {
                    let mut add = BTreeMap::new();
                    for _ in 0..count(&mut r)? {
                        let value = u32le(&mut r)?;
                        let key = string(&mut r)?;
                        if add.insert(key, value).is_some() {
                            return Err(bad());
                        }
                    }
                    let remove = (0..count(&mut r)?)
                        .map(|_| u32le(&mut r))
                        .collect::<io::Result<BTreeSet<_>>>()?;
                    let mut obj = match next.get(&id) {
                        Some(Node::Object(o)) => o.clone(),
                        _ => BTreeMap::new(),
                    };
                    obj.retain(|_, v| !remove.contains(v));
                    obj.extend(add);
                    Node::Object(obj)
                }
                _ => return Err(bad()),
            };
            next.insert(id, node);
            if next.len() > 100_000 {
                return Err(bad());
            }
        }
        if serde_json::to_vec(&next)?.len() > 16 * 1024 * 1024 {
            return Err(bad());
        }
        self.nodes = next;
        Ok(())
    }
    pub fn ready(&self) -> bool {
        if !matches!(self.nodes.get(&0), Some(Node::Object(_))) {
            return false;
        }
        let mut visited = BTreeSet::new();
        let mut pending = vec![0];
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            match self.nodes.get(&id) {
                None => return false,
                Some(Node::Array(a)) => pending.extend(a.iter().copied()),
                Some(Node::Object(o)) => pending.extend(o.values().copied()),
                _ => {}
            }
        }
        true
    }
    pub fn children(&self, id: u32) -> Vec<(String, u32)> {
        match self.nodes.get(&id) {
            Some(Node::Object(o)) => o.iter().map(|(k, v)| (k.clone(), *v)).collect(),
            Some(Node::Array(a)) => a
                .iter()
                .enumerate()
                .map(|(i, v)| (i.to_string(), *v))
                .collect(),
            _ => vec![],
        }
    }
    pub fn view(&self, id: u32) -> Value {
        self.resolve(id, &mut BTreeSet::new(), &mut 5000, 0)
    }
    fn resolve(
        &self,
        id: u32,
        path: &mut BTreeSet<u32>,
        budget: &mut usize,
        depth: usize,
    ) -> Value {
        if *budget == 0 || depth >= 32 {
            return json!("[view limit]");
        }
        *budget -= 1;
        if !path.insert(id) {
            return json!("[cycle]");
        }
        let value = match self.nodes.get(&id) {
            Some(Node::Scalar(v)) => v.clone(),
            Some(Node::Array(a)) => {
                let mut out = Vec::new();
                for child in a {
                    if *budget == 0 {
                        out.push(json!("[view limit]"));
                        break;
                    }
                    out.push(self.resolve(*child, path, budget, depth + 1));
                }
                Value::Array(out)
            }
            Some(Node::Object(o)) => {
                let mut out = serde_json::Map::new();
                for (key, child) in o {
                    if *budget == 0 {
                        break;
                    }
                    out.insert(key.clone(), self.resolve(*child, path, budget, depth + 1));
                }
                Value::Object(out)
            }
            None => json!("[awaiting reference]"),
        };
        path.remove(&id);
        value
    }
}
#[derive(Default)]
pub struct Frames {
    buffer: Vec<u8>,
}
impl Frames {
    pub fn feed(&mut self, data: &[u8]) -> io::Result<Vec<(u8, Vec<u8>)>> {
        if self.buffer.len() + data.len() > MAX_FRAME + 65541 {
            return Err(bad());
        }
        self.buffer.extend_from_slice(data);
        let mut consumed = 0;
        let mut out = Vec::new();
        while self.buffer.len() - consumed >= 5 {
            let h = &self.buffer[consumed..];
            let size = u32::from_le_bytes(h[..4].try_into().unwrap()) as usize;
            if size > MAX_FRAME {
                return Err(bad());
            }
            if h.len() < size + 5 {
                break;
            }
            out.push((h[4], h[5..5 + size].to_vec()));
            consumed += size + 5;
        }
        self.buffer.drain(..consumed);
        Ok(out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn initial() -> Vec<u8> {
        vec![
            8, 0, 0, 0, 0, 1, 0, 1, 0, 0, 0, b'H', b'P', 0, 0, 0, 4, 1, 0, 0, 0, 100, 0, 0, 0,
        ]
    }
    #[test]
    fn database_updates_are_atomic_and_references_stay_live() {
        let mut d = Database::default();
        d.update(&initial()).unwrap();
        assert_eq!(d.view(0)["HP"], 100);
        assert!(d.update(&[4, 1, 0]).is_err());
        assert_eq!(d.view(0)["HP"], 100);
        d.update(&[4, 1, 0, 0, 0, 90, 0, 0, 0]).unwrap();
        assert_eq!(d.view(0)["HP"], 90);
    }
    #[test]
    fn dictionary_removes_old_reference_before_replacing_same_key() {
        let mut d = Database::default();
        d.update(&initial()).unwrap();
        d.update(&[
            8, 0, 0, 0, 0, 1, 0, 2, 0, 0, 0, b'H', b'P', 0, 1, 0, 1, 0, 0, 0, 4, 2, 0, 0, 0, 80, 0,
            0, 0,
        ])
        .unwrap();
        assert_eq!(d.view(0)["HP"], 80);
    }
    #[test]
    fn framing_accepts_every_split_and_multiple_packets() {
        for split in 0..=8 {
            let packet = [3, 0, 0, 0, 1, b'a', b'b', b'c'];
            let mut f = Frames::default();
            let mut out = f.feed(&packet[..split]).unwrap();
            out.extend(f.feed(&packet[split..]).unwrap());
            assert_eq!(out, vec![(1, b"abc".to_vec())]);
        }
        let mut f = Frames::default();
        assert_eq!(f.feed(&[0; 10]).unwrap().len(), 2);
        assert!(f.feed(&[255, 255, 255, 255, 3]).is_err());
    }
    #[test]
    fn cycles_missing_refs_and_invalid_values_are_bounded() {
        let mut d = Database::default();
        d.update(&[7, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0]).unwrap();
        assert_eq!(d.view(0), json!(["[cycle]"]));
        assert_eq!(d.view(42), json!("[awaiting reference]"));
        assert!(d.update(&[5, 1, 0, 0, 0, 0, 0, 128, 127]).is_err());
    }
}
