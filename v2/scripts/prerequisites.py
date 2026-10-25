"""Shared minimal prerequisites, applied in memory; safe to merge with prior ports."""
import re
BUNDLED='''// Only app-bundled paths; arbitrary filesystem paths remain rejected.
pub(super) fn bundled_icon(value: &str) -> bool {
    let Some(name) = value.strip_prefix("/extensions/icons/")
        .or_else(|| value.strip_prefix("/extensions/artwork/")) else { return false };
    let Some((stem, extension)) = name.rsplit_once('.') else { return false };
    !stem.is_empty() && stem.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
        && matches!(extension, "png" | "webp" | "svg")
}

'''
def once(s,old,new):
 if new in s:return s
 if s.count(old)!=1:raise ValueError('Prerequisite anchor missing or ambiguous: '+old[:80])
 return s.replace(old,new,1)
def apply(files):
 p='src-tauri/src/native_core/extensions/v2_catalog.rs';s=files[p]
 if 'fn bundled_icon(' not in s:s=once(s,'pub(super) fn store_item',BUNDLED+'pub(super) fn store_item')
 # Beta already preserves iconUrl; beta.15 clears local paths.
 s=s.replace('if icon_val.starts_with("http") {','if icon_val.starts_with("http") || bundled_icon(icon_val) {')
 files[p]=s
 p='src-tauri/src/native_core/extensions/registry.rs';full=files[p];start=full.index('fn icon_path(');end=full.index('fn icon_data(',start);s=full[start:end]
 if 'name.starts_with("/extensions/")' not in s:s=once(s,'    let relative = Path::new(name.strip_prefix("./").unwrap_or(name));','    if extension.id.starts_with("com.tezbar.native.") && super::v2_catalog::bundled_icon(name) {\n        return Some(name.into());\n    }\n    let relative = Path::new(name.strip_prefix("./").unwrap_or(name));')
 full=full[:start]+s+full[end:];start=full.index('fn icon_data(');s=full[start:];
 if 'path.starts_with("/extensions/")' not in s:s=once(s,'    if path.starts_with("http") {','    if path.starts_with("http") || super::v2_catalog::bundled_icon(&path) {')
 files[p]=full[:start]+s
 p='src/renderer/extensionImages.ts';s=files[p]
 if "if (/^\\/extensions\\/(icons|artwork)\\/[a-z0-9-]+\\.(png|webp|svg)$/.test(value)) return value" not in s:
  s=once(s,"export function extensionImageSource(value: string): string {","export function extensionImageSource(value: string): string {\n  if (/^\\/extensions\\/(icons|artwork)\\/[a-z0-9-]+\\.(png|webp|svg)$/.test(value)) return value")
 files[p]=s
 p='src/renderer/src/raycast-api/detail-runtime.tsx';s=files[p]
 if 'onClick={onRunPrimaryAction}' in s:s=once(s,'onClick={onRunPrimaryAction}','onClick={() => onRunPrimaryAction()}')
 elif 'onClick={() => onRunPrimaryAction()}' not in s:raise ValueError('Detail primary action wrapper needs manual review')
 files[p]=s
 return files
