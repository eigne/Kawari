# Feast native job gauge audit

September 27, 2026. Client 7.51h2, executable pinned by NativeDirector.

## Update path

The server sends an F7 job resource record inside an atomic director frame.
`NativeResourceGauges` requests number-array87 updates, matches the native addon
name and current job, and temporarily sets the shared visibility input during
that addon's refresh. Do not identify addons by vtable address: Dalamud clones
those tables for lifecycle callbacks.

The base JobHud refresh at RVA15A0210 calls virtual81 to populate its data before
rendering. We resolve that reader from the actual addon, validate its address
against the pinned executable and its data bounds against the initialized addon,
then project the server state after the original reader. Both normal and simple
renderers use this data. `NativeJobGaugeData` owns the field mappings. It preserves
the data vtable and BRD's borrowed title pointer. The shared visibility input is
restored in `finally`; outside Feast original readers run unchanged.

`FeastHotbars` also feeds JobGaugeManager for native action consumers. That alone
did not reliably update the HUD in PvP, and is no longer the display mechanism.

## Coverage

| Job | Native display values |
| --- | --- |
| PLD | Oath |
| WAR | Beast |
| DRK | Blood, Darkside timer, Dark Arts |
| GNB | Two cartridges |
| WHM | Lilies, blood lily, ten-second generation |
| SCH | Faerie |
| AST | Lord/Lady crown |
| SGE | Addersgall, ten-second generation, Eukrasia |
| MNK | Chakra |
| DRG | Dragon phase/timer, two stored eyes |
| NIN | Ninki |
| SAM | Kenki, Meditation |
| RPR | Soul, Shroud, Lemure, Void, Enshroud timer |
| BRD | Soul Voice, song, Repertoire, song timer |
| MCH | Heat, Overheat timer |
| DNC | Esprit, feathers, dance steps and progress |
| BLM | Element, element timer, Enochian timer, Polyglot |
| SMN | Aetherflow, trance phase and timer |
| RDM | Black and white mana |

Modern-only gauge segments are disabled: BLM Astral Soul, MNK beast/nadi,
NIN Kazematoi, SAM Sen, SCH's unused Aetherflow panel. Other post-Feast resource
fields are cleared by projection rather than retaining stale PvE values.

BLM uses F7's otherwise-unused resource byte for remaining elemental time in
100ms units (0..150). The existing timer still carries Polyglot time. Jobs.lua
is loaded when the Feast rules engine is created, so a fresh duty instance is
needed to pick up this script change; no server binary/protocol-size change.

## Verification and limits

319 HUD checks include all layout identities/owners, payload bounds, pointer
preservation, nonzero-to-zero transitions and distinct secondary resources.
The memory-loaded assembly test additionally checks all29 payload bounds against
the installed SDK. Historical job engine tests cover BLM timer start/expiry.
Runtime reader activation and unsupported layout errors are logged by addon name.

No automated job switching or computer use was performed. Each job still needs
visual validation in normal/simple mode and after leaving the duty. This reuses
the retained current-client renderers; it does not restore every historical skin.
In particular DRG eyes occupy the current two-counter Focus display, while AST
and SMN use their retained current layouts. The old artwork/layout differences
remain separate from authoritative resource accounting.
