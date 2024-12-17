// Heavily cannibalized from https://github.com/YaLTeR/hldemo-rs

use std::io::{Read, Seek};

pub trait Parse: Sized {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self>;
}

impl Parse for u8 {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let mut bytes = [0u8; 1];
        reader.read_exact(&mut bytes).unwrap();
        Some(bytes[0])
    }
}

impl Parse for i8 {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let mut bytes = [0u8; 1];
        reader.read_exact(&mut bytes).unwrap();
        Some(bytes[0] as i8)
    }
}

impl Parse for i16 {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let mut bytes = [0u8; 2];
        reader.read_exact(&mut bytes).unwrap();
        Some(i16::from_le_bytes(bytes))
    }
}

impl Parse for u16 {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let mut bytes = [0u8; 2];
        reader.read_exact(&mut bytes).unwrap();
        Some(u16::from_le_bytes(bytes))
    }
}

impl Parse for i32 {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes).unwrap();
        Some(i32::from_le_bytes(bytes))
    }
}

impl Parse for u32 {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes).unwrap();
        Some(u32::from_le_bytes(bytes))
    }
}

impl Parse for f32 {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let mut bytes = [0u8; 4];
        reader.read_exact(&mut bytes).unwrap();
        Some(f32::from_le_bytes(bytes))
    }
}

impl<const N: usize> Parse for [u8; N] {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let mut bytes = [0u8; N];
        reader.read_exact(&mut bytes).unwrap();
        Some(bytes)
    }
}

impl<const N: usize> Parse for [f32; N] {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        // TODO: Can't do N * 4 here...
        //let mut bytes = [0u8; N * 4];
        let mut result = [0.0f32; N];
        for i in 0..N {
            result[i] = <f32>::parse(reader)?;
        }
        Some(result)
    }
}

impl<const N: usize> Parse for [i32; N] {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        // TODO: Can't do N * 4 here...
        //let mut bytes = [0u8; N * 4];
        let mut result = [0i32; N];
        for i in 0..N {
            result[i] = <i32>::parse(reader)?;
        }
        Some(result)
    }
}

macro_rules! parsable_struct {
    ($struct_name:ident { $(  $field_name:ident : $field_ty:ty  ),*$(,)* })=> {
        #[derive(Clone, Debug, PartialEq)]
        pub struct $struct_name {
            $(
                pub $field_name: $field_ty,
            )*
        }

        impl Parse for $struct_name {
            fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
                $(
                    let $field_name = <$field_ty>::parse(reader)?;
                )*
                Some(Self {
                    $(
                        $field_name,
                    )*
                })
            }
        }
    };
}

parsable_struct!(DemoHeader {
    magic: [u8; 8],
    demo_protocol: i32,
    network_protocol: i32,
    map_name: [u8; 260],
    game_directory: [u8; 260],
    map_checksum: u32,
    directory_offset: u32,
});

parsable_struct!(DemoEntry {
    entry_ty: i32,
    description: [u8; 64],
    flags: i32,
    cd_track: i32,
    track_time: i32,
    frame_count: i32,
    offset: i32,
    file_len: i32,
});

#[derive(Clone, Debug)]
pub struct DemoDirectory {
    pub len: i32,
    pub entries: Vec<DemoEntry>,
}

impl Parse for DemoDirectory {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let len = <i32>::parse(reader)?;
        let mut entries = Vec::new();
        for _ in 0..len {
            entries.push(DemoEntry::parse(reader)?);
        }
        Some(Self { len, entries })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DemoFrameType {
    NetMsg(u8),
    DemoStart,
    ConsoleCommand,
    ClientData,
    NextSection,
    Event,
    WeaponAnim,
    Sound,
    DemoBuffer,
}

impl Parse for DemoFrameType {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let data = <[u8; 1]>::parse(reader)?;
        Some(match data[0] {
            2 => Self::DemoStart,
            3 => Self::ConsoleCommand,
            4 => Self::ClientData,
            5 => Self::NextSection,
            6 => Self::Event,
            7 => Self::WeaponAnim,
            8 => Self::Sound,
            9 => Self::DemoBuffer,
            x => Self::NetMsg(x),
        })
    }
}

parsable_struct!(DemoFrameHeader {
    frame_ty: DemoFrameType,
    time: f32,
    frame: i32,
});

parsable_struct!(RefParams {
    view_org: [f32; 3],
    view_angles: [f32; 3],
    forward: [f32; 3],
    right: [f32; 3],
    up: [f32; 3],
    frame_time: f32,
    time: f32,
    intermission: i32,
    paused: i32,
    spectator: i32,
    onground: i32,
    waterlevel: i32,
    simvel: [f32; 3],
    simorg: [f32; 3],
    viewheight: [f32; 3],
    idealpitch: f32,
    cl_viewangles: [f32; 3],
    health: i32,
    crosshairangle: [f32; 3],
    viewsize: f32,
    punchangle: [f32; 3],
    maxclients: i32,
    viewentity: i32,
    playernum: i32,
    max_entities: i32,
    demoplayback: i32,
    hardware: i32,
    smoothing: i32,
    ptr_cmd: i32,
    ptr_movevars: i32,
    viewport: [i32; 4],
    next_view: i32,
    only_client_draw: i32,
});

parsable_struct!(UserCmd {
    lerp_msec: i16,
    msec: u8,
    _padding_1: [u8; 1],
    viewangles: [f32; 3],
    forwardmove: f32,
    sidemove: f32,
    upmove: f32,
    lightlevel: i8,
    _padding_2: [u8; 1],
    buttons: u16,
    impulse: i8,
    weaponselect: i8,
    _padding_3: [u8; 2],
    impact_index: i32,
    impact_position: [f32; 3],
});

parsable_struct!(MoveVars {
    gravity: f32,
    stopspeed: f32,
    maxspeed: f32,
    spectatormaxspeed: f32,
    accelerate: f32,
    airaccelerate: f32,
    wateraccelerate: f32,
    friction: f32,
    edgefriction: f32,
    waterfriction: f32,
    entgravity: f32,
    bounce: f32,
    stepsize: f32,
    maxvelocity: f32,
    zmax: f32,
    wave_height: f32,
    footsteps: i32,
    sky_name: [u8; 32],
    rollangle: f32,
    rollspeed: f32,
    skycolor_r: f32,
    skycolor_g: f32,
    skycolor_b: f32,
    skyvec_x: f32,
    skyvec_y: f32,
    skyvec_z: f32,
});

parsable_struct!(NetMsgInfo {
    timestamp: f32,
    ref_params: RefParams,
    user_cmd: UserCmd,
    move_vars: MoveVars,
    view: [f32; 3],
    view_model: i32,
});

parsable_struct!(NetMsgDataPrefix {
    info: NetMsgInfo,
    incoming_sequence: i32,
    incoming_acknowledged: i32,
    incoming_reliable_acknowledged: i32,
    incoming_reliable_sequence: i32,
    outgoing_sequence: i32,
    reliable_sequence: i32,
    last_reliable_sequence: i32,
    msg_len: i32,
});

#[derive(Clone, Debug, PartialEq)]
pub struct NetMsgData {
    pub prefix: NetMsgDataPrefix,
    pub msg: Vec<u8>,
}

impl Parse for NetMsgData {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let prefix = <NetMsgDataPrefix>::parse(reader)?;
        let mut msg = vec![0u8; prefix.msg_len as usize];
        reader.read_exact(&mut msg).unwrap();
        Some(Self { prefix, msg })
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum NetMsgFrameType {
    /// Initialization frames.
    Start,
    /// Normal frames.
    Normal,
    /// Never emitted by the official engine but parsed as NetMsg nevertheless.
    Unknown(u8),
}

impl NetMsgFrameType {
    fn from_raw(frame_ty: u8) -> Option<Self> {
        match frame_ty {
            0 => Some(NetMsgFrameType::Start),
            1 => Some(NetMsgFrameType::Normal),
            2..=9 => None,
            x => Some(NetMsgFrameType::Unknown(x)),
        }
    }
}

parsable_struct!(ConsoleCommandData { data: [u8; 64] });

parsable_struct!(ClientDataData {
    origin: [f32; 3],
    viewangles: [f32; 3],
    weapon_bits: i32,
    fov: f32,
});

parsable_struct!(EventData {
    flags: i32,
    index: i32,
    delay: f32,
    args: EventArgs,
});

parsable_struct!(EventArgs {
    flags: i32,
    entity_index: i32,
    origin: [f32; 3],
    angles: [f32; 3],
    velocity: [f32; 3],
    ducking: i32,
    fparam1: f32,
    fparam2: f32,
    iparam1: i32,
    iparam2: i32,
    bparam1: i32,
    bparam2: i32,
});

parsable_struct!(WeaponAnimData {
    anim: i32,
    body: i32,
});

#[derive(Clone, Debug, PartialEq)]
pub struct BytesWithLength {
    pub data: Vec<u8>,
}

impl Parse for BytesWithLength {
    fn parse<T: Seek + Read>(reader: &mut T) -> Option<Self> {
        let len = <i32>::parse(reader)?;
        let mut data = vec![0u8; len as usize];
        reader.read_exact(&mut data).unwrap();
        Some(Self { data })
    }
}

parsable_struct!(SoundData {
    channel: i32,
    sample: BytesWithLength,
    attenuation: f32,
    volume: f32,
    flags: i32,
    pitch: i32,
});

parsable_struct!(DemoBufferData {
    data: BytesWithLength,
});

#[derive(Debug, PartialEq)]
pub enum DemoFrameData {
    NetMsg((NetMsgFrameType, NetMsgData)),
    DemoStart,
    ConsoleCommand(ConsoleCommandData),
    ClientData(ClientDataData),
    NextSection,
    Event(EventData),
    WeaponAnim(WeaponAnimData),
    Sound(SoundData),
    DemoBuffer(DemoBufferData),
}

pub fn parse_demo_frame_data<T: Seek + Read>(
    reader: &mut T,
    frame_ty: DemoFrameType,
) -> Option<DemoFrameData> {
    Some(match frame_ty {
        DemoFrameType::NetMsg(x) => {
            DemoFrameData::NetMsg((NetMsgFrameType::from_raw(x)?, NetMsgData::parse(reader)?))
        }
        DemoFrameType::DemoStart => DemoFrameData::DemoStart,
        DemoFrameType::ConsoleCommand => {
            DemoFrameData::ConsoleCommand(ConsoleCommandData::parse(reader)?)
        }
        DemoFrameType::ClientData => DemoFrameData::ClientData(ClientDataData::parse(reader)?),
        DemoFrameType::NextSection => DemoFrameData::NextSection,
        DemoFrameType::Event => DemoFrameData::Event(EventData::parse(reader)?),
        DemoFrameType::WeaponAnim => DemoFrameData::WeaponAnim(WeaponAnimData::parse(reader)?),
        DemoFrameType::Sound => DemoFrameData::Sound(SoundData::parse(reader)?),
        DemoFrameType::DemoBuffer => DemoFrameData::DemoBuffer(DemoBufferData::parse(reader)?),
    })
}
