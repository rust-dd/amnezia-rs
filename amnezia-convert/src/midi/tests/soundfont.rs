use super::*;

const SAMPLES: u32 = 1024;

/// An in-memory sine-wave bank keeps synthesis coverage independent of local assets.
pub(super) fn fixture() -> Arc<SoundFont> {
    let mut samples = (0..SAMPLES)
        .flat_map(|index| {
            let phase = std::f32::consts::TAU * index as f32 / 64.0;
            ((phase.sin() * 8192.0) as i16).to_le_bytes()
        })
        .collect::<Vec<_>>();
    samples.extend_from_slice(&[0; 92]);

    let mut presets = named_record::<38>(b"Sine").to_vec();
    let mut terminal_preset = named_record::<38>(b"EOP");
    terminal_preset[24..26].copy_from_slice(&1_u16.to_le_bytes());
    presets.extend_from_slice(&terminal_preset);

    let mut instruments = named_record::<22>(b"Sine").to_vec();
    let mut terminal_instrument = named_record::<22>(b"EOI");
    terminal_instrument[20..22].copy_from_slice(&1_u16.to_le_bytes());
    instruments.extend_from_slice(&terminal_instrument);

    let mut sample = named_record::<46>(b"Sine");
    for (offset, value) in [(20, 0), (24, SAMPLES), (28, 0), (32, SAMPLES), (36, 44_100)] {
        sample[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
    sample[40] = 60;
    sample[44..46].copy_from_slice(&1_u16.to_le_bytes());
    let mut headers = sample.to_vec();
    headers.extend_from_slice(&named_record::<46>(b"EOS"));

    let info = list(b"INFO", [chunk(b"ifil", &[2, 0, 1, 0])]);
    let data = list(b"sdta", [chunk(b"smpl", &samples)]);
    let parameters = list(
        b"pdta",
        [
            chunk(b"phdr", &presets),
            chunk(b"pbag", &[0, 0, 0, 0, 1, 0, 0, 0]),
            chunk(b"pmod", &[0; 10]),
            chunk(b"pgen", &[41, 0, 0, 0, 0, 0, 0, 0]),
            chunk(b"inst", &instruments),
            chunk(b"ibag", &[0, 0, 0, 0, 2, 0, 0, 0]),
            chunk(b"imod", &[0; 10]),
            chunk(b"igen", &[54, 0, 1, 0, 53, 0, 0, 0, 0, 0, 0, 0]),
            chunk(b"shdr", &headers),
        ],
    );
    let bytes = chunk(
        b"RIFF",
        &[b"sfbk".as_slice(), &info, &data, &parameters].concat(),
    );
    Arc::new(SoundFont::new(&mut Cursor::new(bytes)).unwrap())
}

fn named_record<const N: usize>(name: &[u8]) -> [u8; N] {
    let mut record = [0; N];
    record[..name.len()].copy_from_slice(name);
    record
}

fn list(kind: &[u8; 4], chunks: impl IntoIterator<Item = Vec<u8>>) -> Vec<u8> {
    let mut payload = kind.to_vec();
    for data in chunks {
        payload.extend(data);
    }
    chunk(b"LIST", &payload)
}

fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
    let mut bytes = id.to_vec();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(payload);
    if !payload.len().is_multiple_of(2) {
        bytes.push(0);
    }
    bytes
}
