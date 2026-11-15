# Third-party licences and how each tool is used

Every tool below is free and open source. "Linked" means the code is imported
into the same Python process as the HELQIX modules. "Separate process" means
the HELQIX code only writes a file (or passes arguments) and reads the output;
no library from that tool is loaded into the HELQIX process. Licences were
read from the project's own metadata, site or repository on 2026-11-15 unless
marked UNKNOWN.

## Libraries linked into the Python process

| Tool | Version | Licence | Verified from | How used |
| --- | --- | --- | --- | --- |
| Python | 3.11 in CI, 3.13.12 in the local venv | PSF License Version 2 | https://docs.python.org/3/license.html ("PYTHON SOFTWARE FOUNDATION LICENSE VERSION 2") | interpreter |
| numpy | 2.5.4 | BSD-3-Clause AND 0BSD AND MIT AND Zlib AND CC0-1.0 | `numpy-2.5.4.dist-info/METADATA`, `License-Expression` field of the installed wheel | array maths in `helqix_circuit` and `helqix_sensors` |
| scipy | 1.18.1 | BSD-3-Clause (and BSD-3-Clause-Open-MPI for bundled components) | `scipy-1.18.1.dist-info/LICENSE.txt` of the installed wheel | sparse LU solve (`helqix_circuit.mna`, `core`), root finding and interpolation |
| pytest | 9.2.0 | MIT | `pytest-9.2.0.dist-info/METADATA`, `License-Expression` field | test runner only; never imported by the packages |

Nothing is GPL-linked: no copyleft library is imported by the HELQIX
packages, so the BSD/MIT/PSF terms apply as attribution only.

## Tools run as a separate process

| Tool | Version | Licence | Verified from | How used |
| --- | --- | --- | --- | --- |
| ngspice | 42+ds-3build1 on ubuntu-latest (the apt package, MEASURED from the CI log); the project site currently offers 47 | modified BSD (manual CC-BY-SA 4.0) | ngspice FAQ, https://ngspice.sourceforge.io/faq.html ("The ngspice license is the modified BSD license"), read 2026-11-15 | circuit cross-check only: `scripts/ngspice_crosscheck.py` writes a deck and runs `ngspice -b`; no ngspice code is linked or imported. Local runs skip when ngspice is absent. |
| git | local install, version not recorded | GPL-2.0 | `COPYING` in https://github.com/git/git (v2 only is valid for that project) | version control; not part of the shipped module |
| curl | local install, version not recorded | curl (MIT/X-inspired, SPDX `curl`) | https://curl.se/docs/copyright.html | GitHub API calls for pull requests and Actions logs; not part of the shipped module |
| GitHub Actions `actions/checkout` | tag `v4` | MIT | `LICENSE` in https://github.com/actions/checkout | CI checkout |
| GitHub Actions `actions/setup-python` | tag `v5` | MIT | `LICENSE` in https://github.com/actions/setup-python | CI Python install |
| browser-use (Qoder MCP server) | version not recorded | MIT | `LICENSE` in https://github.com/browser-use/browser-use | read-only rendering of published web pages while auditing the ampacity table; the tool is supplied by the editor and no code is added to the repository |

## Not used

- No proprietary simulator, compiler or netlist tool is required.
- No licence manager, no paid API and no vendored binary is stored in the
  repository; the ngspice deck exporter is plain text written by
  `helqix_circuit.spice`.
- No tool above was used to generate values that the module prints as its own
  results. ngspice is an independent check only; the module never calls it at
  run time.

## Honesty notes

- The ngspice version actually used by CI is 42+ds-3build1 (read from the
  apt line and the `--version` banner in the job log of run 43703077659).
- "Modified BSD" is the wording of the ngspice project itself. The summary of
  the three merged simulators (spice3f5 and CIDER modified BSD, XSPICE public
  domain) is copied from that FAQ, not independently audited file by file.
- Versions of git, curl and browser-use were not recorded; only their
  licences were read from the projects' own files.
