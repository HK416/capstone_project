use std::{
    fs::File,
    io::{self, BufReader, BufWriter, Read, Write},
    path::{Path, PathBuf},
    sync::Arc,
};

use ahash::RandomState;
use dashmap::DashMap;

use crate::error::PathNotFound;

/// ## Cached Asset Data
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CachedAsset {
    filename: PathBuf,
    bytes: Vec<u8>,
}

impl CachedAsset {
    /// 파일 이름을 가져옵니다.
    pub fn filename(&self) -> &Path {
        &self.filename
    }

    /// 바이트 배열을 가져옵니다.
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// ## Asset Manager (Inner)
#[derive(Debug)]
pub struct AssetManagerInner {
    root_dir: PathBuf,
    cached: DashMap<PathBuf, Arc<CachedAsset>, RandomState>,
}

/// ## Asset Manager
#[derive(Debug, Clone)]
pub struct AssetManager(Arc<AssetManagerInner>);

impl AssetManager {
    /// 새로운 에셋 관리자를 생성합니다.  
    /// 최상위 디렉토리 경로를 찾지 못한 경우 `PathNotFound`를 반환합니다.
    pub fn new<P>(root_dir: P) -> Result<Self, PathNotFound>
    where
        P: Into<PathBuf>,
    {
        let root_dir: PathBuf = root_dir.into();
        if !root_dir.is_dir() {
            return Err(PathNotFound(root_dir));
        }

        Ok(Self(Arc::new(AssetManagerInner {
            root_dir,
            cached: DashMap::default(),
        })))
    }

    /// 에셋의 최상위 디렉토리 경로를 가져옵니다.
    pub fn get_root_dir(&self) -> &Path {
        &self.0.root_dir
    }

    /// 에셋 번들에 파일을 캐싱합니다.
    /// 이 함수는 항상 파일에서 데이터를 읽어 에셋 번들에 캐싱합니다.
    ///
    /// 이미 에셋 번들에 캐싱되어 있는 경우 새로 읽은 데이터로 교체됩니다.
    ///
    pub fn load<P>(&self, path: P) -> Result<Arc<CachedAsset>, io::Error>
    where
        P: Into<PathBuf>,
    {
        // 에셋 파일의 경로를 생성합니다.
        let relative_path: PathBuf = path.into();
        let mut absolute_path = self.get_root_dir().to_path_buf();
        absolute_path.push(relative_path.clone());

        // 파일 핸들을 생성하고, 파일을 엽니다.
        let file = File::open(absolute_path)?;

        // 파일 내용을 읽습니다.
        let mut reader = BufReader::new(file);
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;

        // 에셋 캐쉬를 생성합니다.
        let cache = Arc::new(CachedAsset {
            filename: relative_path.clone(),
            bytes: data,
        });

        self.0.cached.insert(relative_path, cache.clone());
        Ok(cache)
    }

    pub fn create<P>(&self, path: P, data: &[u8]) -> Result<Arc<CachedAsset>, io::Error>
    where
        P: Into<PathBuf>,
    {
        // 에셋 파일의 경로를 생성합니다.
        let relative_path: PathBuf = path.into();
        let mut absolute_path = self.get_root_dir().to_path_buf();
        absolute_path.push(relative_path.clone());

        // 파일을 생성합니다. 파일이 이미 존재하는 경우 오류를 발생시킵니다.
        let file = File::create_new(absolute_path)?;

        // 파일에 내용을 작성합니다.
        let mut writer = BufWriter::new(file);
        writer.write_all(data)?;

        // 에셋 캐쉬를 생성합니다.
        let cache = Arc::new(CachedAsset {
            filename: relative_path.clone(),
            bytes: data.to_vec(),
        });

        self.0.cached.insert(relative_path, cache.clone());
        Ok(cache)
    }

    /// 에셋 번들에 주어진 경로에 해당하는 캐싱된 에셋을 가져옵니다.  
    /// 해당 에셋이 존재하지 않는 경우 에셋을 로드하고, 에셋 번들에 캐싱합니다.
    pub fn get_or_init<P>(&self, path: P) -> Result<Arc<CachedAsset>, io::Error>
    where
        P: Into<PathBuf>,
    {
        let path: PathBuf = path.into();
        match self.0.cached.get(&path) {
            Some(guard) => Ok(guard.clone()),
            None => self.load(path),
        }
    }

    /// 에셋 번들에 주어진 경로에 해당하는 캐싱된 에셋을 제거합니다.  
    /// 해당 에셋이 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    pub fn remove<P>(&self, path: P) -> Option<Arc<CachedAsset>>
    where
        P: Into<PathBuf>,
    {
        self.0.cached.remove(&path.into()).map(|(_, cached)| cached)
    }
}
