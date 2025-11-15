#![allow(dead_code)]
//! 사운드 에셋과 관련된 코드를 관리합니다.
//!

use std::{
    fs::OpenOptions,
    io::{Cursor, Read},
    path::Path,
    sync::Arc,
    time::Duration,
};

use ahash::{HashMap, RandomState};
use parking_lot::{FairMutex, FairMutexGuard};
use rodio::{ChannelCount, Decoder, SampleRate, Source, decoder::DecoderError};

use crate::asset::AssetError;

/// 디코딩된 사운드 데이터를 저장하는 구조체
#[derive(Debug, Clone)]
pub struct DecodedSound {
    samples: Arc<Vec<f32>>,
    sample_rate: u32,
    channels: u16,
}

impl DecodedSound {
    pub fn from_bytes(data: &'static [u8]) -> Result<Self, DecoderError> {
        let cursor = Cursor::new(data);
        let decoder = Decoder::new(cursor)?;

        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let samples: Vec<f32> = decoder.collect();

        Ok(Self {
            samples: Arc::new(samples),
            sample_rate,
            channels,
        })
    }

    pub fn from_vec(data: Vec<u8>) -> Result<Self, DecoderError> {
        let cursor = Cursor::new(data);
        let decoder = Decoder::new(cursor)?;

        let sample_rate = decoder.sample_rate();
        let channels = decoder.channels();
        let samples: Vec<f32> = decoder.collect();

        Ok(Self {
            samples: Arc::new(samples),
            sample_rate,
            channels,
        })
    }

    pub fn as_source(&self) -> DecodedSoundSource {
        DecodedSoundSource {
            samples: self.samples.clone(),
            position: 0,
            sample_rate: self.sample_rate,
            channels: self.channels,
        }
    }
}

/// radio Source trait를 구현하는 구조체
#[derive(Debug, Clone)]
pub struct DecodedSoundSource {
    samples: Arc<Vec<f32>>,
    position: usize,
    sample_rate: u32,
    channels: u16,
}

impl Source for DecodedSoundSource {
    fn current_span_len(&self) -> Option<usize> {
        Some(self.samples.len() - self.position)
    }

    fn channels(&self) -> ChannelCount {
        self.channels
    }

    fn sample_rate(&self) -> SampleRate {
        self.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let samples_per_second = self.sample_rate as f64 * self.channels as f64;
        let duration_seconds = self.samples.len() as f64 / samples_per_second;
        Some(Duration::from_secs_f64(duration_seconds))
    }
}

impl Iterator for DecodedSoundSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        if self.position < self.samples.len() {
            let sample = self.samples[self.position];
            self.position += 1;
            Some(sample)
        } else {
            None
        }
    }
}

/// 로드된 사운드 데이터를 관리하는 풀 객체입니다.
#[derive(Debug, Clone)]
pub struct SoundDataPool(Arc<FairMutex<SoundDataPoolType>>);

pub type SoundDataPoolType = HashMap<String, DecodedSound>;

/// 사운드 데이터 풀 객체의 용량입니다.
pub const SOUND_DATA_POOL_CAPACITY: usize = 64;

impl SoundDataPool {
    /// 새로운 풀 객체를 생성합니다.
    pub fn new() -> Self {
        Self(Arc::new(FairMutex::new(HashMap::with_capacity_and_hasher(
            SOUND_DATA_POOL_CAPACITY,
            RandomState::new(),
        ))))
    }

    /// 풀 객체의 `lock`을 획득합니다.
    ///
    /// # Warning
    /// `FairMutexGuard`가 지속되는 동안 풀 객체의 다른 함수를 호출하면 데드락이 발생합니다.
    ///
    pub fn lock(&self) -> FairMutexGuard<'_, SoundDataPoolType> {
        self.0.lock()
    }

    /// 파일로부터 사운드 데이터를 가져옵니다.
    fn load_from_file<Dir, Uri>(workspace: Dir, uri: Uri) -> Result<Vec<u8>, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        let mut path = workspace.as_ref().to_path_buf();
        path.push(format!("{}.ogg", uri.as_ref()));

        log::debug!("open texture pixel asset (PATH:{})", path.display());
        let mut file = OpenOptions::new()
            .read(true)
            .write(false)
            .open(&path)
            .map_err(|e| {
                log::error!(
                    "failed to open texture pixel asset (PATH:{}, REASON:{})",
                    path.display(),
                    &e
                );
                AssetError::IOError(e)
            })?;

        log::debug!("read texture pixel asset (PATH:{})", path.display());
        let mut buf = Vec::new();
        file.read_to_end(&mut buf).map_err(|e| {
            log::error!(
                "failed to read texture pixel asset (PATH:{}, REASON:{})",
                path.display(),
                &e
            );
            AssetError::IOError(e)
        })?;

        log::debug!("close texture pixel asset (PATH:{})", path.display());
        drop(file);

        return Ok(buf);
    }

    /// 사운드 데이터 풀 객체에 등록된 사운드 데이터를 가져옵니다.  
    /// 해당 Uri에 등록된 사운드 데이터가 없는 경우 사운드 데이터를 새로 생성합니다.
    pub fn get_or_init<Dir, Uri>(
        &self,
        workspace: Dir,
        uri: Uri,
    ) -> Result<DecodedSound, AssetError>
    where
        Dir: AsRef<Path>,
        Uri: AsRef<str>,
    {
        // 풀 객체를 가져옵니다.
        let mut pool = self.lock();

        if let Some(data) = pool.get(uri.as_ref()).cloned() {
            return Ok(data);
        }

        // 사운드 데이터를 로드합니다.
        let data = Self::load_from_file(workspace.as_ref(), uri.as_ref())?;
        let decoded = DecodedSound::from_vec(data)?;

        // 생성된 사운드 데이터를 풀 객체에 등록합니다.
        pool.insert(uri.as_ref().to_string(), decoded.clone());

        Ok(decoded)
    }

    /// 주어진 Uri에 해당하는 사운드 데이터를 풀 객체에서 가져옵니다.
    /// 해당 사운드 데이터가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn get<Uri>(&self, uri: Uri) -> Option<DecodedSound>
    where
        Uri: AsRef<str>,
    {
        self.lock().get(uri.as_ref()).cloned()
    }

    /// 사운드 데이터 풀 객체에 사운드 데이터를 등록합니다.  
    /// 이미 Uri에 해당하는 사운드 데이터가 존재할 경우 기존의 사운드 데이터를 반환합니다.
    pub fn insert<Uri>(&self, uri: Uri, decoded: DecodedSound) -> Option<DecodedSound>
    where
        Uri: AsRef<str>,
    {
        self.lock().insert(uri.as_ref().into(), decoded)
    }

    /// 주어진 Uri에 해당하는 사운드 데이터를 풀 객체에서 제거합니다.  
    /// 해당 사운드 데이터가 풀 객체에 존재하지 않는 경우 `None`을 반환합니다.
    pub fn remove<Uri>(&self, uri: Uri) -> Option<DecodedSound>
    where
        Uri: AsRef<str>,
    {
        self.lock().remove(uri.as_ref()).map(|item| item)
    }

    /// 풀 객체에 존재하는 모든 사운드 데이터를 제거합니다.
    pub fn clear(&self) {
        self.lock().clear()
    }
}
