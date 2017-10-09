pub struct SourceItem {
    pub offset: usize,
    pub length: usize,
    pub file_index: usize,
}

pub enum JumpType {
    FunctionIn,
    FunctionOut,
    Regular
}

pub struct SourceMapItem {
    pub source: SourceItem,
    pub jump: Option<JumpType>,
}

// TODO: Do not panic in parsing error.

pub fn parse_source(s: &str) -> Vec<SourceItem> {
    let mut ret = Vec::new();
    let mut last = 0;
    for item in s.split(';') {
        let mut values: Vec<usize> = Vec::new();
        for raw in item.split(':') {
            if raw.is_empty() {
                values.push(last);
            } else {
                let value = raw.parse().unwrap();
                values.push(value);
                last = value;
            }
        }

        while values.len() < 3 {
            values.push(last);
        }
        ret.push(SourceItem { offset: values[0], length: values[1], file_index: values[2] });
    }
    ret
}

pub fn parse_source_map(s: &str) -> Vec<SourceMapItem> {
    let mut ret = Vec::new();
    let mut last = 0;
    for item in s.split(';') {
        let mut values: Vec<usize> = Vec::new();
        let mut jump_value = None;
        for raw in item.split(':') {
            if values.len() > 3 {
                jump_value = Some(raw);
                break;
            }
            if raw.is_empty() {
                values.push(last);
            } else {
                let value = raw.parse().unwrap();
                values.push(value);
                last = value;
            }
        }

        while values.len() < 3 {
            values.push(last);
        }
        ret.push(SourceMapItem {
            source: SourceItem { offset: values[0], length: values[1], file_index: values[2] },
            jump: jump_value.map(|jump_value| {
                match jump_value {
                    "i" => JumpType::FunctionIn,
                    "o" => JumpType::FunctionOut,
                    "-" => JumpType::Regular,
                    _ => panic!(),
                }
            }),
        });
    }
    ret
}
