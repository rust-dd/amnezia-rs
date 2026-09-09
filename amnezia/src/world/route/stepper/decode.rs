use amnezia_data::MoveCommandDef;

/// MoveEvent embeds bytes in its integer array: command IDs are single bytes,
/// but integer arguments and CP1250 string lengths use big-endian 7-bit groups.
/// Reject an incomplete route as a whole so its prefix cannot mutate the scene.
pub(super) fn commands(stream: &[i32]) -> Option<Vec<MoveCommandDef>> {
    let mut cursor = Packed(stream);
    let mut commands = Vec::new();
    while !cursor.0.is_empty() {
        let code = u32::from(cursor.byte()?);
        let (params, string) = match code {
            32 | 33 => (vec![cursor.int()?], String::new()),
            34 | 35 => {
                let name = cursor.name()?;
                let count = if code == 34 { 1 } else { 3 };
                let params = (0..count)
                    .map(|_| cursor.int())
                    .collect::<Option<Vec<_>>>()?;
                (params, name)
            }
            _ => (Vec::new(), String::new()),
        };
        commands.push(MoveCommandDef {
            code,
            params,
            string,
        });
    }
    Some(commands)
}

struct Packed<'a>(&'a [i32]);

impl Packed<'_> {
    fn byte(&mut self) -> Option<u8> {
        let (value, tail) = self.0.split_first()?;
        self.0 = tail;
        u8::try_from(*value).ok()
    }

    fn int(&mut self) -> Option<i32> {
        let mut value = 0_u32;
        for _ in 0..5 {
            let byte = self.byte()?;
            value = value.checked_mul(128)?.checked_add(u32::from(byte & 127))?;
            if byte & 128 == 0 {
                return Some(value as i32);
            }
        }
        None
    }

    fn name(&mut self) -> Option<String> {
        let len = usize::try_from(self.int()?).ok()?;
        let bytes = self
            .0
            .get(..len)?
            .iter()
            .map(|&byte| u8::try_from(byte).ok())
            .collect::<Option<Vec<_>>>()?;
        self.0 = &self.0[len..];
        Some(encoding_rs::WINDOWS_1250.decode(&bytes).0.into_owned())
    }
}
