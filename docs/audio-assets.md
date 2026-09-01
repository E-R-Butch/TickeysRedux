# Bundled Audio Asset Provenance

This inventory records what the repository can currently establish about the bundled sound packs. It is not a new license grant. The project's MIT license covers the Redux source code; it does not automatically relicense third-party audio files.

| Sound pack | Evidence currently in the repository | Recorded source/license | Status |
|---|---|---|---|
| `bubble` | `assets/data/bubble/license.txt` | Glaneur de sons on Freesound; individual files recorded as CC BY 3.0 | Documented |
| `drum` | `assets/data/drum/_readme_and_license.txt` | Veiler on Freesound; listed source files recorded as CC0 | Documented |
| `mechanical` | `assets/data/mechanical/license.txt` | jim-ph on Freesound; listed source files recorded as CC0 | Documented |
| `Cherry_G80_3000` | No source or license file found | Unknown | Needs provenance review |
| `Cherry_G80_3494` | No source or license file found | Unknown | Needs provenance review |
| `sword` | No source or license file found | Unknown | Needs provenance review |
| `typewriter` | No source or license file found | Unknown | Needs provenance review |
| `starwars` | No source or license file found | Unknown | Needs provenance review |

“Unknown” means that the current project files do not establish origin or redistribution terms. It does not mean that a pack is public domain, unlicensed, or necessarily unusable.

Before the next public release:

1. Trace each undocumented pack to an authoritative source and record the author, source URL, exact license, attribution text, and any modifications.
2. Replace or remove files whose redistribution terms cannot be established.
3. Generate a third-party notices file and include it, the project `LICENSE`, and applicable per-pack license files in the App Bundle, DMG, and ZIP.
4. Keep this inventory synchronized whenever an audio file is added, replaced, renamed, or normalized.

## 中文说明

本表只记录仓库目前能够证明的音效来源，不构成新的授权。项目 MIT 许可证适用于 Redux 源代码，不会自动把第三方音频重新许可为 MIT。

“Needs provenance review”表示现有项目文件不足以证明来源或再分发条款；它不等于音效一定侵权，也不等于可以按公有领域使用。下次公开发布前，应补齐作者、权威来源、准确许可证、署名文字和修改记录；无法确认再分发条件的文件应替换或移除。
