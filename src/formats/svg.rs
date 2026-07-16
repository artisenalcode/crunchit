use anyhow::Result;
use std::fs;
use std::path::Path;

pub(crate) fn optimize(path: &Path, _lossy: bool) -> Result<u64> {
    let initial_size = fs::metadata(path)?.len();
    let content = fs::read_to_string(path)?;

    use oxvg_ast::{parse::roxmltree::parse, serialize::Node as _, visitor::Info};
    use oxvg_optimiser::Jobs;

    let optimized_content = parse(&content, |dom, allocator| {
        let jobs = Jobs::default();
        jobs.run(dom, &Info::new(allocator)).unwrap();
        dom.serialize().unwrap()
    })
    .map_err(|e| anyhow::anyhow!("svg parse error: {}", e))?;

    if optimized_content.len() < initial_size as usize {
        fs::write(path, optimized_content)?;
        Ok(initial_size - fs::metadata(path)?.len())
    } else {
        Ok(0)
    }
}
