//! Multi-voice mechanical keyboard sound synthesizer for MINDFORGE.
//!
//! Provides authentic mechanical switch audio feedback with zero latency:
//! - 6 synthesized sound profiles (Thocky, Clacky, Creamy, Marbly, Poppy, Clicky)
//! - 4-channel hardware audio mixer (Windows winmm) for overlapping keystrokes
//! - 3 pitch variations per profile to prevent "machine gun" repetition
//! - Sample rate: 22050 Hz, 16-bit PCM mono
//!
//! Each profile is synthesized at startup using additive synthesis with
//! exponential decay envelopes — no audio files needed.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Available mechanical keyboard sound profiles.
pub enum SoundProfile {
    Off,
    Thocky,
    Clacky,
    Creamy,
    Marbly,
    Poppy,
    Clicky,
}

impl SoundProfile {
    pub const ALL: [SoundProfile; 7] = [
        SoundProfile::Off,
        SoundProfile::Thocky,
        SoundProfile::Clacky,
        SoundProfile::Creamy,
        SoundProfile::Marbly,
        SoundProfile::Poppy,
        SoundProfile::Clicky,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            SoundProfile::Off => "Off",
            SoundProfile::Thocky => "Thocky",
            SoundProfile::Clacky => "Clacky",
            SoundProfile::Creamy => "Creamy",
            SoundProfile::Marbly => "Marbly",
            SoundProfile::Poppy => "Poppy",
            SoundProfile::Clicky => "Clicky",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            SoundProfile::Off => "Silent typing",
            SoundProfile::Thocky => "Deep, muted, low-pitched satisfying thud",
            SoundProfile::Clacky => "Sharper, crisp bottom-out sound",
            SoundProfile::Creamy => "Smooth marble-like pop (popular in ASMR)",
            SoundProfile::Marbly => "Bright, resonant tapping glass sound",
            SoundProfile::Poppy => "Bright & snappy with a distinct bubbly pop",
            SoundProfile::Clicky => "Sharp mechanical click with click-bar action",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "off" | "none" | "silent" => Some(SoundProfile::Off),
            "thocky" | "thock" => Some(SoundProfile::Thocky),
            "clacky" | "clack" => Some(SoundProfile::Clacky),
            "creamy" | "cream" => Some(SoundProfile::Creamy),
            "marbly" | "marble" => Some(SoundProfile::Marbly),
            "poppy" | "pop" => Some(SoundProfile::Poppy),
            "clicky" | "click" => Some(SoundProfile::Clicky),
            _ => None,
        }
    }
}

/// Main sound engine that manages profile selection and playback.
///
/// Pre-synthesizes 3 pitch variations of each profile at startup to
/// avoid runtime synthesis cost. The `var_idx` cycles through variations
/// on each keystroke for natural-sounding repetition.
pub struct SoundEngine {
    /// Currently active sound profile
    pub profile: SoundProfile,
    /// Pre-synthesized Thocky samples at 3 pitch levels (0.95x, 1.0x, 1.05x)
    thocky: [Vec<i16>; 3],
    /// Pre-synthesized Clacky samples at 3 pitch levels
    clacky: [Vec<i16>; 3],
    /// Pre-synthesized Creamy samples at 3 pitch levels
    creamy: [Vec<i16>; 3],
    /// Pre-synthesized Marbly samples at 3 pitch levels
    marbly: [Vec<i16>; 3],
    /// Pre-synthesized Poppy samples at 3 pitch levels
    poppy: [Vec<i16>; 3],
    /// Pre-synthesized Clicky samples at 3 pitch levels
    clicky: [Vec<i16>; 3],
    /// Cycles 0→1→2→0 to select pitch variation
    var_idx: usize,
    /// Hardware audio mixer for overlapping playback
    player: MultiVoicePlayer,
}

impl SoundEngine {
    pub fn new(profile: SoundProfile) -> Self {
        let sr = 22050;
        Self {
            profile,
            thocky: [
                synthesize_thocky(sr, 1.0),
                synthesize_thocky(sr, 0.95),
                synthesize_thocky(sr, 1.05),
            ],
            clacky: [
                synthesize_clacky(sr, 1.0),
                synthesize_clacky(sr, 0.96),
                synthesize_clacky(sr, 1.04),
            ],
            creamy: [
                synthesize_creamy(sr, 1.0),
                synthesize_creamy(sr, 0.97),
                synthesize_creamy(sr, 1.03),
            ],
            marbly: [
                synthesize_marbly(sr, 1.0),
                synthesize_marbly(sr, 0.96),
                synthesize_marbly(sr, 1.04),
            ],
            poppy: [
                synthesize_poppy(sr, 1.0),
                synthesize_poppy(sr, 0.95),
                synthesize_poppy(sr, 1.05),
            ],
            clicky: [
                synthesize_clicky(sr, 1.0),
                synthesize_clicky(sr, 0.97),
                synthesize_clicky(sr, 1.03),
            ],
            var_idx: 0,
            player: MultiVoicePlayer::new(sr),
        }
    }

    /// Plays a single keystroke sound using the active profile.
    ///
    /// Cycles through 3 pitch variations to prevent repetitive "machine gun"
    /// effect. Routes the PCM sample to the next available hardware channel,
    /// allowing up to 4 simultaneous keystroke sounds.
    pub fn play(&mut self) {
        if self.profile == SoundProfile::Off {
            return;
        }

        self.var_idx = (self.var_idx + 1) % 3;
        let idx = self.var_idx;

        let samples = match self.profile {
            SoundProfile::Off => return,
            SoundProfile::Thocky => &self.thocky[idx],
            SoundProfile::Clacky => &self.clacky[idx],
            SoundProfile::Creamy => &self.creamy[idx],
            SoundProfile::Marbly => &self.marbly[idx],
            SoundProfile::Poppy => &self.poppy[idx],
            SoundProfile::Clicky => &self.clicky[idx],
        };

        self.player.play(samples);
    }
}

// --- MULTI-VOICE HARDWARE AUDIO MIXER ---

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct WaveFormatEx {
    w_format_tag: u16,
    n_channels: u16,
    n_samples_per_sec: u32,
    n_avg_bytes_per_sec: u32,
    n_block_align: u16,
    w_bits_per_sample: u16,
    cb_size: u16,
}

#[cfg(windows)]
#[repr(C)]
#[derive(Clone, Copy)]
struct WaveHdr {
    lp_data: *mut u8,
    dw_buffer_length: u32,
    dw_bytes_recorded: u32,
    dw_user: usize,
    dw_flags: u32,
    dw_loops: u32,
    lp_next: usize,
    reserved: usize,
}

#[cfg(windows)]
impl Default for WaveHdr {
    fn default() -> Self {
        Self {
            lp_data: std::ptr::null_mut(),
            dw_buffer_length: 0,
            dw_bytes_recorded: 0,
            dw_user: 0,
            dw_flags: 0,
            dw_loops: 0,
            lp_next: 0,
            reserved: 0,
        }
    }
}

#[cfg(windows)]
#[link(name = "winmm")]
extern "system" {
    fn waveOutOpen(
        phwo: *mut usize,
        u_device_id: u32,
        pwfx: *const WaveFormatEx,
        dw_callback: usize,
        dw_instance: usize,
        fdw_open: u32,
    ) -> u32;
    fn waveOutPrepareHeader(hwo: usize, pwh: *mut WaveHdr, cbwh: u32) -> u32;
    fn waveOutUnprepareHeader(hwo: usize, pwh: *mut WaveHdr, cbwh: u32) -> u32;
    fn waveOutWrite(hwo: usize, pwh: *mut WaveHdr, cbwh: u32) -> u32;
    fn waveOutReset(hwo: usize) -> u32;
    fn waveOutClose(hwo: usize) -> u32;
}

#[cfg(windows)]
const NUM_CHANNELS: usize = 4;

struct MultiVoicePlayer {
    #[cfg(windows)]
    hwos: [usize; NUM_CHANNELS],
    #[cfg(windows)]
    headers: [WaveHdr; NUM_CHANNELS],
    #[cfg(windows)]
    prepared: [bool; NUM_CHANNELS],
    #[allow(dead_code)]
    channel_idx: usize,
}

impl MultiVoicePlayer {
    pub fn new(_sample_rate: u32) -> Self {
        #[cfg(windows)]
        {
            let sample_rate = _sample_rate;
            let wfx = WaveFormatEx {
                w_format_tag: 1, // WAVE_FORMAT_PCM
                n_channels: 1,
                n_samples_per_sec: sample_rate,
                n_avg_bytes_per_sec: sample_rate * 2,
                n_block_align: 2,
                w_bits_per_sample: 16,
                cb_size: 0,
            };
            let mut hwos = [0usize; NUM_CHANNELS];
            for i in 0..NUM_CHANNELS {
                let mut hwo: usize = 0;
                let res = unsafe { waveOutOpen(&mut hwo, 0xFFFFFFFF, &wfx, 0, 0, 0) };
                if res == 0 {
                    hwos[i] = hwo;
                }
            }
            Self {
                hwos,
                headers: [WaveHdr::default(); NUM_CHANNELS],
                prepared: [false; NUM_CHANNELS],
                channel_idx: 0,
            }
        }
        #[cfg(not(windows))]
        {
            Self { channel_idx: 0 }
        }
    }

    pub fn play(&mut self, samples: &[i16]) {
        if samples.is_empty() {
            return;
        }
        #[cfg(windows)]
        {
            self.channel_idx = (self.channel_idx + 1) % NUM_CHANNELS;
            let idx = self.channel_idx;
            let hwo = self.hwos[idx];
            if hwo == 0 {
                return;
            }
            let hdr_size = std::mem::size_of::<WaveHdr>() as u32;

            unsafe {
                // If this channel has an active buffer, reset it so it never buffers behind
                if self.prepared[idx] {
                    waveOutReset(hwo);
                    waveOutUnprepareHeader(hwo, &mut self.headers[idx], hdr_size);
                    self.prepared[idx] = false;
                }
                self.headers[idx].lp_data = samples.as_ptr() as *mut u8;
                self.headers[idx].dw_buffer_length = (samples.len() * 2) as u32;
                self.headers[idx].dw_flags = 0;

                if waveOutPrepareHeader(hwo, &mut self.headers[idx], hdr_size) == 0 {
                    if waveOutWrite(hwo, &mut self.headers[idx], hdr_size) == 0 {
                        self.prepared[idx] = true;
                    }
                }
            }
        }
    }
}

impl Drop for MultiVoicePlayer {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            let hdr_size = std::mem::size_of::<WaveHdr>() as u32;
            for i in 0..NUM_CHANNELS {
                let hwo = self.hwos[i];
                if hwo != 0 {
                    if self.prepared[i] {
                        unsafe {
                            waveOutReset(hwo);
                            waveOutUnprepareHeader(hwo, &mut self.headers[i], hdr_size);
                        }
                    }
                    unsafe {
                        waveOutClose(hwo);
                    }
                }
            }
        }
    }
}

// --- SYNTHESIS ALGORITHMS ---

/// 1. Thocky: A deep, muted, and satisfying low-pitched sound (~110Hz with damped sub-harmonics).
fn synthesize_thocky(sr: u32, pitch: f32) -> Vec<i16> {
    let dur = (0.048 * sr as f32) as usize;
    let mut out = Vec::with_capacity(dur);
    let base_f = 110.0 * pitch;

    for i in 0..dur {
        let t = i as f32 / sr as f32;
        let env = (-t * 85.0).exp();
        let wave = (2.0 * std::f32::consts::PI * base_f * t).sin() * 0.7
            + (2.0 * std::f32::consts::PI * (base_f * 0.55) * t).sin() * 0.5
            + (2.0 * std::f32::consts::PI * (base_f * 1.8) * t).sin() * 0.2;
        let sample = (wave * env * 24000.0).clamp(-32000.0, 32000.0) as i16;
        out.push(sample);
    }
    out
}

/// 2. Clacky: A sharper, higher-pitched, and crisp bottom-out sound (~880Hz).
fn synthesize_clacky(sr: u32, pitch: f32) -> Vec<i16> {
    let dur = (0.030 * sr as f32) as usize;
    let mut out = Vec::with_capacity(dur);
    let base_f = 880.0 * pitch;

    for i in 0..dur {
        let t = i as f32 / sr as f32;
        let env = (-t * 140.0).exp();
        let wave = (2.0 * std::f32::consts::PI * base_f * t).sin() * 0.6
            + (2.0 * std::f32::consts::PI * (base_f * 1.5) * t).sin() * 0.35
            + (2.0 * std::f32::consts::PI * (base_f * 2.2) * t).sin() * 0.2;
        let sample = (wave * env * 22000.0).clamp(-32000.0, 32000.0) as i16;
        out.push(sample);
    }
    out
}

/// 3. Creamy: A smooth, high-pitched, and marble-like pop sound popular in ASMR.
fn synthesize_creamy(sr: u32, pitch: f32) -> Vec<i16> {
    let dur = (0.038 * sr as f32) as usize;
    let mut out = Vec::with_capacity(dur);
    let base_f = 620.0 * pitch;

    for i in 0..dur {
        let t = i as f32 / sr as f32;
        let attack = (t / 0.003).min(1.0);
        let env = attack * (-t * 90.0).exp();
        let wave = (2.0 * std::f32::consts::PI * base_f * t).sin() * 0.8
            + (2.0 * std::f32::consts::PI * (base_f * 1.95) * t).sin() * 0.35;
        let sample = (wave * env * 25000.0).clamp(-32000.0, 32000.0) as i16;
        out.push(sample);
    }
    out
}

/// 4. Marbly: A bright, resonant sound resembling tapping glass or small marbles (~1750Hz dual chime).
fn synthesize_marbly(sr: u32, pitch: f32) -> Vec<i16> {
    let dur = (0.036 * sr as f32) as usize;
    let mut out = Vec::with_capacity(dur);
    let f1 = 1750.0 * pitch;
    let f2 = 2420.0 * pitch;

    for i in 0..dur {
        let t = i as f32 / sr as f32;
        let env = (-t * 110.0).exp();
        let wave = (2.0 * std::f32::consts::PI * f1 * t).sin() * 0.55
            + (2.0 * std::f32::consts::PI * f2 * t).sin() * 0.45;
        let sample = (wave * env * 21000.0).clamp(-32000.0, 32000.0) as i16;
        out.push(sample);
    }
    out
}

/// 5. Poppy: A bright and snappy sound with a distinct "pop" on each keystroke (downward pitch sweep).
fn synthesize_poppy(sr: u32, pitch: f32) -> Vec<i16> {
    let dur = (0.032 * sr as f32) as usize;
    let mut out = Vec::with_capacity(dur);

    for i in 0..dur {
        let t = i as f32 / sr as f32;
        let env = (-t * 125.0).exp();
        let sweep_f = (950.0 - 670.0 * (t / 0.025).min(1.0)) * pitch;
        let wave = (2.0 * std::f32::consts::PI * sweep_f * t).sin();
        let sample = (wave * env * 26000.0).clamp(-32000.0, 32000.0) as i16;
        out.push(sample);
    }
    out
}

/// 6. Clicky: A sharp, high-pitched mechanical click created by specialized click jackets or click bars.
fn synthesize_clicky(sr: u32, pitch: f32) -> Vec<i16> {
    let dur = (0.026 * sr as f32) as usize;
    let mut out = Vec::with_capacity(dur);
    let click_f = 3400.0 * pitch;
    let body_f = 600.0 * pitch;

    for i in 0..dur {
        let t = i as f32 / sr as f32;
        let snap_env = (-t * 350.0).exp();
        let body_env = (-t * 110.0).exp();
        let wave = (2.0 * std::f32::consts::PI * click_f * t).sin() * snap_env * 0.7
            + (2.0 * std::f32::consts::PI * body_f * t).sin() * body_env * 0.4;
        let sample = (wave * 25000.0).clamp(-32000.0, 32000.0) as i16;
        out.push(sample);
    }
    out
}
