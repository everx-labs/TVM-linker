use regex::Regex;
use std::convert::TryFrom;
use std::fmt::{LowerHex, UpperHex, Display};

lazy_static! {
    pub static ref NAMES: Regex = Regex::new(r"\$(?P<id>:?[-_0-9a-zA-Z\.]+)(?P<offset>\+\d+)?(:(?P<len>\d*)?(?P<fmt>[xX])?)?\$").unwrap();
}

pub fn resolve_name<F, T>(text: &str, mut get: F) -> Result<String, String> 
    where 
        F: FnMut(&str) -> Option<T>,
        T: LowerHex + UpperHex + Display + TryFrom<isize> + std::ops::AddAssign {
    let mut res_str = String::new();
    let mut end = 0;
    for cap in NAMES.captures_iter(text) {
        if cap.name("id").is_none() {
            return Err("invalid syntax: object name not found".to_string());
        }
        let name_match = cap.name("id").unwrap();
        res_str += text.get(end..name_match.start() - 1).unwrap();
        let name = name_match.as_str();

        let offset = cap.name("offset").map(|m| {
                let off_str = m.as_str();
                let off = isize::from_str_radix(off_str.get(1..).unwrap(), 10).unwrap();
                if off_str.starts_with('-') { 0 - off } else { off }
            }).unwrap_or(0);

        let mut id = get(name).ok_or(format!("name \"{}\" not found", name))?;
        id += T::try_from(offset).map_err(|_| "symbol offset is too big".to_string())?;

        let len = match cap.name("len") {
            Some(len_match) => {
                let l = len_match.as_str();
                if l == "" { "0" } else { l }
            },
            None => "0",
        };

        let total_len = usize::from_str_radix(len, 10).map_err(|_| format!("width modifier ({}) is invalid", len))?;

        let fmt = match cap.name("fmt") {
            Some(fmt_match) => fmt_match.as_str(),
            None => "",
        };
        let id_str = match fmt {
            "x" => format!("{:0width$x}", id, width = total_len),
            "X" => format!("{:0width$X}", id, width = total_len),
            _   => format!("{:0width$}",  id, width = total_len),
        };

        res_str += &id_str;
        end = cap.get(0).unwrap().end();
    }
    res_str += text.get(end..).unwrap();
    Ok(res_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    lazy_static! {
        static ref MAP: HashMap<String, u32> = {
            let mut map = HashMap::new();
            map.insert("ctor".to_string(), 0x112233FF);
            map.insert("ctor_1".to_string(), 0x1111);
            map.insert("get".to_string(), 0xFF);
            map.insert(":int".to_string(), 10);
            map.insert("x.y".to_string(), 11);
            map
        };
    }

    fn id_by_name(name: &str) -> Option<u32> {
        MAP.get(name).map(|x| x.clone())
    }
    
    #[test]
    fn test_resolve_simple() {
        assert_eq!(resolve_name("$ctor$", id_by_name),      Ok("287454207".to_string()));
        assert_eq!(resolve_name("00$ctor$", id_by_name),    Ok("00287454207".to_string()));
        assert_eq!(resolve_name("$ctor_1$end", id_by_name), Ok("4369end".to_string()));
        assert_eq!(resolve_name("$:int$", id_by_name),      Ok("10".to_string()));
        assert_eq!(resolve_name("$x.y$", id_by_name),      Ok("11".to_string()));
    }

    #[test]
    fn test_resolve_x() {
        assert_eq!(resolve_name("$ctor:x$", id_by_name), Ok("112233ff".to_string()));
        assert_eq!(resolve_name("$ctor:X$", id_by_name), Ok("112233FF".to_string()));
        assert_eq!(resolve_name("$:int:x$", id_by_name), Ok("a".to_string()));
        assert_eq!(resolve_name("qwerty", id_by_name),   Ok("qwerty".to_string()));
        assert_eq!(resolve_name("$x.y:x$", id_by_name),  Ok("b".to_string()));
    }

    #[test]
    fn test_resolve_unknown() {
        assert_eq!(resolve_name("00$unknown$", id_by_name).is_err(), true);
    }

    #[test]
    fn test_resolve_len() {
        assert_eq!(resolve_name("1$get:08X$2", id_by_name), Ok("1000000FF2".to_string()));
        assert_eq!(resolve_name("$get:02x$", id_by_name),   Ok("ff".to_string()));
        assert_eq!(resolve_name("$ctor:02x$", id_by_name),  Ok("112233ff".to_string()));
        assert_eq!(resolve_name("$:int:011X$", id_by_name), Ok("0000000000A".to_string()));
        assert_eq!(resolve_name("$x.y:04x$", id_by_name),      Ok("000b".to_string()));
    }

    #[test]
    fn test_resolve_multiple() {
        assert_eq!(resolve_name(" $ctor:X$---$get$$ctor_1:08x$", id_by_name), Ok(" 112233FF---25500001111".to_string()));
    }

    #[test]
    fn test_resolve_offset() {
        assert_eq!(resolve_name("$ctor+16$", id_by_name), Ok("287454223".to_string()));
        assert_eq!(resolve_name("$ctor+0$", id_by_name), Ok("287454207".to_string()));
        assert_eq!(resolve_name("$ctor+16:X$", id_by_name), Ok("1122340F".to_string()));
        assert_eq!(resolve_name("$get+256:08x$", id_by_name), Ok("000001ff".to_string()));
        assert_eq!(resolve_name("$ctor+$", id_by_name), Ok("$ctor+$".to_string()));
        assert_eq!(resolve_name("$x.y+1$", id_by_name),      Ok("12".to_string()));
    }
}