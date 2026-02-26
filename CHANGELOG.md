# Changelog

## [0.5.2](https://github.com/brwse/earl/compare/v0.5.1...v0.5.2) (2026-02-26)


### Features

* add allow_private_ips config option for homelab/self-hosted services ([#71](https://github.com/brwse/earl/issues/71)) ([9a79efb](https://github.com/brwse/earl/commit/9a79efba6dac613d2508a7d28e3e65b6786c35d6))


### Bug Fixes

* prevent bash injection in examples and clarify SQL injection risk ([#68](https://github.com/brwse/earl/issues/68)) ([2b8f0de](https://github.com/brwse/earl/commit/2b8f0de9d7f0c8de1369c778055c9022a083f776))

## [0.5.1](https://github.com/brwse/earl/compare/v0.5.0...v0.5.1) (2026-02-26)


### Features

* add earl runtime skill and reorganize skills by category ([#66](https://github.com/brwse/earl/issues/66)) ([9bec065](https://github.com/brwse/earl/commit/9bec06510536ad6d3bb11d5119f19cdd3fe939aa))
* **browser:** add browser protocol with full Playwright MCP parity ([#59](https://github.com/brwse/earl/issues/59)) ([65f01da](https://github.com/brwse/earl/commit/65f01daf7a3c2ad63a737cd075d06fa06f975bd7))
* **browser:** default to 'default' session when session_id is omitted ([#63](https://github.com/brwse/earl/issues/63)) ([99c3a66](https://github.com/brwse/earl/commit/99c3a66e2be92c951b48f3c948961b168435b46e))


### Bug Fixes

* add earl-protocol-browser to release-please config and manifest ([#67](https://github.com/brwse/earl/issues/67)) ([6f18bc2](https://github.com/brwse/earl/commit/6f18bc24d1563225559a9e82a79b6f30e8528335))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * earl-core bumped from 0.5.0 to 0.5.1
    * earl-protocol-grpc bumped from 0.5.0 to 0.5.1
    * earl-protocol-http bumped from 0.5.0 to 0.5.1
    * earl-protocol-bash bumped from 0.5.0 to 0.5.1
    * earl-protocol-sql bumped from 0.5.0 to 0.5.1
    * earl-protocol-browser bumped from 0.5.0 to 0.5.1

## [0.5.0](https://github.com/brwse/earl/compare/v0.4.1...v0.5.0) (2026-02-24)


### ⚠ BREAKING CHANGES

* handle optional params gracefully and validate template args ([#49](https://github.com/brwse/earl/issues/49))

### Features

* add named environments support ([#41](https://github.com/brwse/earl/issues/41)) ([98773b1](https://github.com/brwse/earl/commit/98773b17f44a3a6e7461f66e5cb3d9676e1f62e7))
* **bash:** add memory and CPU resource limits to sandbox ([#39](https://github.com/brwse/earl/issues/39)) ([6f57817](https://github.com/brwse/earl/commit/6f578170cf12b53f7e1d597a74d3546e203aa3c5))
* compiled catalog cache for faster CLI startup ([#34](https://github.com/brwse/earl/issues/34)) ([0349a86](https://github.com/brwse/earl/commit/0349a86973b003a7c532190737e334e4142664ac))
* earl agent skills suite (setup-earl, create-template, migrate-to-earl, troubleshoot-earl, secure-agent) ([#40](https://github.com/brwse/earl/issues/40)) ([c75a399](https://github.com/brwse/earl/commit/c75a399023d2c5072b39c84941660c0a6e7a61ea))
* external secret manager support ([#43](https://github.com/brwse/earl/issues/43)) ([e6afe3c](https://github.com/brwse/earl/commit/e6afe3cef58a65c3bdd62ed7f530eb1ce34b2f69))
* handle optional params gracefully and validate template args ([#49](https://github.com/brwse/earl/issues/49)) ([b0f1654](https://github.com/brwse/earl/commit/b0f1654616c4f19cd7f586904bfe986fee148112))
* recall.ai integration (14-command template + agent skill) ([#44](https://github.com/brwse/earl/issues/44)) ([4fee573](https://github.com/brwse/earl/commit/4fee573dd2ff14801be5453ac7972e7f70ccbe9e))


### Bug Fixes

* serialize onepassword env-var tests to prevent parallel races ([#54](https://github.com/brwse/earl/issues/54)) ([427d689](https://github.com/brwse/earl/commit/427d6892c85f7fcb3631657ec207f1e8895b8afa))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * earl-core bumped from 0.4.1 to 0.5.0
    * earl-protocol-grpc bumped from 0.4.1 to 0.5.0
    * earl-protocol-http bumped from 0.4.1 to 0.5.0
    * earl-protocol-bash bumped from 0.4.1 to 0.5.0
    * earl-protocol-sql bumped from 0.4.1 to 0.5.0

## [0.4.1](https://github.com/brwse/earl/compare/v0.4.0...v0.4.1) (2026-02-23)


### Features

* add 25 pre-built company API templates ([#16](https://github.com/brwse/earl/issues/16)) ([f59fded](https://github.com/brwse/earl/commit/f59fdedc1615c4d80b5c490c5c93b3512cf2e08b))
* add streaming support for HTTP, gRPC, and Bash ([#17](https://github.com/brwse/earl/issues/17)) ([a35dc6b](https://github.com/brwse/earl/commit/a35dc6b66c8fd094592ac767acff45531dc7ada5))
* enhance GitHub release notes with Claude ([#20](https://github.com/brwse/earl/issues/20)) ([936afc2](https://github.com/brwse/earl/commit/936afc2f3737c6bb9cd8e4430f5b2b41ec5c5833))


### Bug Fixes

* resolve latest CLI release from monorepo releases ([#15](https://github.com/brwse/earl/issues/15)) ([ef651a7](https://github.com/brwse/earl/commit/ef651a7c4162d41db82e7cf8692b0937edb11427))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * earl-core bumped from 0.4.0 to 0.4.1
    * earl-protocol-grpc bumped from 0.4.0 to 0.4.1
    * earl-protocol-http bumped from 0.4.0 to 0.4.1
    * earl-protocol-bash bumped from 0.4.0 to 0.4.1
    * earl-protocol-sql bumped from 0.4.0 to 0.4.1

## [0.4.0](https://github.com/brwse/earl/compare/v0.3.0...v0.4.0) (2026-02-22)


### Features

* add meta Earl example template ([3263ce7](https://github.com/brwse/earl/commit/3263ce701cfce3ff20cb6c1bc8f7f8c4e425c02f))
* JWT authentication and policy engine for MCP server ([#3](https://github.com/brwse/earl/issues/3)) ([f0dc326](https://github.com/brwse/earl/commit/f0dc326eccdfa5ad5c5f0892353ff8545b31b5f1))


### Bug Fixes

* address CodeQL security alerts ([#6](https://github.com/brwse/earl/issues/6)) ([dafe732](https://github.com/brwse/earl/commit/dafe732cf89622e0c9ff436200bd17bb871ba4bd))
* remove include-component-in-tag from sub-crates ([#11](https://github.com/brwse/earl/issues/11)) ([5aed777](https://github.com/brwse/earl/commit/5aed7774cd2a204e41352c5dde2fdd2bec16a810))
* remove invalid matrix reference from CodeQL concurrency group ([#8](https://github.com/brwse/earl/issues/8)) ([e6b7817](https://github.com/brwse/earl/commit/e6b78171b959fafc9d6c90189aca68bc667a24a6))


### Dependencies

* The following workspace dependencies were updated
  * dependencies
    * earl-core bumped from 0.3.0 to 0.4.0
    * earl-protocol-grpc bumped from 0.3.0 to 0.4.0
    * earl-protocol-http bumped from 0.3.0 to 0.4.0
    * earl-protocol-bash bumped from 0.3.0 to 0.4.0
    * earl-protocol-sql bumped from 0.3.0 to 0.4.0

## [0.3.0](https://github.com/brwse/earl/compare/v0.2.0...v0.3.0) (2026-02-22)

### Features

- initial commit ([aa93512](https://github.com/brwse/earl/commit/aa93512873c38519c457296dff1c8eed8fcbe947))

### Dependencies

- The following workspace dependencies were updated
  - dependencies
    - earl-core bumped from 0.2.0 to 0.3.0
    - earl-protocol-grpc bumped from 0.2.0 to 0.3.0
    - earl-protocol-http bumped from 0.2.0 to 0.3.0
    - earl-protocol-bash bumped from 0.2.0 to 0.3.0
    - earl-protocol-sql bumped from 0.2.0 to 0.3.0
