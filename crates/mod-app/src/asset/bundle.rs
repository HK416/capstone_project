use std::{fs::File, io::{BufReader, Read}, path::{Path, PathBuf}, sync::Arc};

use mod_parallelism::collections::SkipMap;

use super::AssetError;



/// 캐싱된 에셋 데이터입니다.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CachedAsset {
    /// 에셋 파일의 이름입니다.
    filename: PathBuf, 

    /// 바이트 배열로 된 에셋 파일의 내용입니다.
    bytes: Vec<u8>, 
}



/// 에셋 번들의 내부 데이터입니다.
pub struct AssetBundleInner {
    /// 에셋의 루트 디렉토리 경로입니다.
    root_dir: PathBuf, 

    /// 캐싱된 에셋의 집합입니다.
    cached: SkipMap<PathBuf, Arc<CachedAsset>>
}



/// 애플리케이션 에셋을 관리하는 관리자입니다.
#[derive(Clone)]
pub struct AssetBundle(Arc<AssetBundleInner>);

impl AssetBundle {
    /// 새로운 에셋 관리자를 생성합니다.
    #[must_use]
    pub fn new<P: Into<PathBuf>>(root_dir: P) -> Result<Self, AssetError> {
        // 에셋의 최상위 디렉토리를 가져옵니다.
        let root_dir: PathBuf = root_dir.into();

        // 에셋의 최상위 디렉토리가 존재하는지 확인합니다.
        if !root_dir.is_dir() {
            return Err(AssetError::PathNotFound(root_dir));
        }

        Ok(Self(AssetBundleInner {
            root_dir, 
            cached: SkipMap::new()
        }.into()))
    }

    /// 에셋의 최상위 디렉토리 경로를 가져옵니다.
    #[inline]
    #[must_use]
    pub fn get_root_dir(&self) -> &Path {
        &self.0.root_dir
    }

    /// 에셋 번들에 파일을 캐싱합니다.
    /// 이 함수는 항상 파일에서 데이터를 읽어 에셋 번들에 캐싱합니다.
    /// 
    /// 이미 에셋 번들에 캐싱되어 있는 경우 새로 읽은 데이터로 교체됩니다.
    /// 
    pub fn load<P: Into<PathBuf>>(&self, path: P) -> Result<Arc<CachedAsset>, AssetError> {
        // 에셋 파일의 경로를 생성합니다.
        let filename: PathBuf = path.into();
        let mut path = self.get_root_dir().to_path_buf();
        path.push(filename.clone());

        // 에셋 파일이 존재하는지 확인합니다.
        if !path.is_file() {
            return Err(AssetError::PathNotFound(path));
        }

        // 파일 핸들을 생성하고, 파일을 엽니다.
        let file = File::open(path).map_err(|e| AssetError::from(e))?;
        let mut reader = BufReader::new(file);
        
        // 파일 내용을 읽습니다.
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes)
            .map_err(|e| AssetError::from(e))?;

        // 에셋 캐쉬를 생성합니다.
        let cache = Arc::new(CachedAsset { filename: filename.clone(), bytes });
        
        // 에셋 번들에 캐싱된 에셋을 추가합니다.
        let cache_cloned = cache.clone();
        self.0.cached.insert(filename, cache_cloned);
        Ok(cache)
    }

    /// 에셋 번들에 주어진 경로에 해당하는 캐싱된 에셋을 가져옵니다.
    /// 
    /// 해당 에셋이 존재하지 않는 경우 에셋을 로드하고, 에셋 번들에 캐싱합니다.
    /// 
    #[inline]
    #[must_use]
    pub fn get_or_init<P: AsRef<PathBuf>>(&self, path: P) -> Result<Arc<CachedAsset>, AssetError> {
        match self.0.cached.get(path.as_ref()) {
            Some(guard) => Ok(guard.clone()), 
            None => self.load(path.as_ref())
        }
    }

    /// 에셋 번들에 주어진 경로에 해당하는 캐싱된 에셋을 제거합니다.
    /// 
    /// 해당 에셋이 존재하지 않는 경우 아무 동작을 수행하지 않습니다.
    /// 
    #[inline]
    pub fn remove<P: AsRef<PathBuf>>(&self, path: P) -> Option<Arc<CachedAsset>> {
        self.0.cached.remove(path.as_ref())
    }
}

impl std::fmt::Debug for AssetBundle {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(stringify!(AssetBundle))
            .field("root_dir", &self.0.root_dir)
            .finish()
    }
}
