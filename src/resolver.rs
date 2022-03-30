/*
 * Copyright 2018-2022 TON DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */
use failure::{bail, format_err};
use regex::Regex;
use std::convert::TryFrom;
use std::fmt::{LowerHex, UpperHex, Display};
use ton_labs_assembler::Line;
use ton_types::Result;

lazy_static! {
    pub static ref NAMES: Regex = Regex::new(r"\$(?P<id>:?[-_0-9a-zA-Z\.]+)(?P<offset>\+\d+)?(:(?P<len>\d*)?(?P<fmt>[xX])?)?\$").unwrap();
}

pub fn resolve_name<F, T>(line: &Line, mut get: F) -> Result<Line>
    where
        F: FnMut(&str) -> Option<T>,
        T: LowerHex + UpperHex + Display + TryFrom<isize> + std::ops::AddAssign {
    let mut res_str = String::new();
    let mut end = 0;
    let semicolon_pos = line.text.find(';').unwrap_or(line.text.len());
    let (text_old, text_rem) = line.text.split_at(semicolon_pos);
    if !text_old.contains('$') {
        return Ok(line.clone());
    }
    for cap in NAMES.captures_iter(text_old) {
        if cap.name("id").is_none() {
           bail!("invalid syntax: object name not found");
        }
        let name_match = cap.name("id").unwrap();
        res_str += text_old.get(end..name_match.start() - 1).unwrap();
        let name = name_match.as_str();

        let offset = cap.name("offset").map(|m| {
            let off_str = m.as_str();
            let off = off_str.get(1..).unwrap().parse::<isize>().unwrap();
            if off_str.starts_with('-') { 0 - off } else { off }
        }).unwrap_or(0);

        let mut id = get(name).ok_or_else(|| format_err!("name \"{}\" not found", name))?;
        id += T::try_from(offset).map_err(|_| format_err!("symbol offset is too big"))?;

        let len = match cap.name("len") {
            Some(len_match) => {
                let l = len_match.as_str();
                if l.is_empty() { "0" } else { l }
            },
            None => "0",
        };

        let total_len = len.parse::<usize>()
            .map_err(|_| format_err!("width modifier ({}) is invalid", len))?;

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
    res_str += text_old.get(end..).unwrap();
    if !res_str.ends_with(' ') && !text_rem.is_empty() {
        res_str += " ";
    }
    res_str += text_rem;
    let res = Line::new(res_str.as_str(), line.pos.filename.as_str(), line.pos.line);
    Ok(res)
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

    pub fn resolve_name<F, T>(text: &str, get: F) -> Result<String>
        where
            F: FnMut(&str) -> Option<T>,
            T: LowerHex + UpperHex + Display + TryFrom<isize> + std::ops::AddAssign {
        let line = Line::new(text, "", 0);
        let res = super::resolve_name(&line, get);
        res.map(|lines| {
            lines.text.clone()
        })
    }

    #[test]
    fn test_resolve_simple() {
        assert_eq!(resolve_name("$ctor$", id_by_name).unwrap(),      "287454207");
        assert_eq!(resolve_name("00$ctor$", id_by_name).unwrap(),    "00287454207");
        assert_eq!(resolve_name("$ctor_1$end", id_by_name).unwrap(), "4369end");
        assert_eq!(resolve_name("$:int$", id_by_name).unwrap(),      "10");
        assert_eq!(resolve_name("$x.y$", id_by_name).unwrap(),       "11");
    }

    #[test]
    fn test_resolve_x() {
        assert_eq!(resolve_name("$ctor:x$", id_by_name).unwrap(), "112233ff");
        assert_eq!(resolve_name("$ctor:X$", id_by_name).unwrap(), "112233FF");
        assert_eq!(resolve_name("$:int:x$", id_by_name).unwrap(), "a");
        assert_eq!(resolve_name("qwerty", id_by_name).unwrap(),   "qwerty");
        assert_eq!(resolve_name("$x.y:x$", id_by_name).unwrap(),  "b");
    }

    #[test]
    fn test_resolve_unknown() {
        assert_eq!(resolve_name("00$unknown$", id_by_name).is_err(), true);
    }

    #[test]
    fn test_resolve_len() {
        assert_eq!(resolve_name("1$get:08X$2", id_by_name).unwrap(), "1000000FF2");
        assert_eq!(resolve_name("$get:02x$", id_by_name).unwrap(),   "ff");
        assert_eq!(resolve_name("$ctor:02x$", id_by_name).unwrap(),  "112233ff");
        assert_eq!(resolve_name("$:int:011X$", id_by_name).unwrap(), "0000000000A");
        assert_eq!(resolve_name("$x.y:04x$", id_by_name).unwrap(),   "000b");
    }

    #[test]
    fn test_resolve_multiple() {
        assert_eq!(resolve_name(" $ctor:X$---$get$$ctor_1:08x$", id_by_name).unwrap(), " 112233FF---25500001111");
    }

    #[test]
    fn test_resolve_offset() {
        assert_eq!(resolve_name("$ctor+16$", id_by_name).unwrap(),     "287454223");
        assert_eq!(resolve_name("$ctor+0$", id_by_name).unwrap(),      "287454207");
        assert_eq!(resolve_name("$ctor+16:X$", id_by_name).unwrap(),   "1122340F");
        assert_eq!(resolve_name("$get+256:08x$", id_by_name).unwrap(), "000001ff");
        assert_eq!(resolve_name("$ctor+$", id_by_name).unwrap(),       "$ctor+$");
        assert_eq!(resolve_name("$x.y+1$", id_by_name).unwrap(),       "12");
    }

    #[test]
    fn test_resolve_with_comments() {
        assert_eq!(resolve_name("text ; ignore this $ctor$", id_by_name).unwrap(), "text ; ignore this $ctor$");
    }
}
