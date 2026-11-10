# Stillharbour documentation

These pages describe the implemented application and the rules for working on it.
The default build is a development product with original-file execution gates
closed; implemented experimental code is not an enabled production capability.

## Use the application

| Document                                          | Purpose                                                              |
| ------------------------------------------------- | -------------------------------------------------------------------- |
| [User guide](USER_GUIDE.md)                       | Browsing, annotations, albums, maps, inspections and troubleshooting |
| [Build capabilities](CODEX_SPEC.md)               | What the default build provides and what remains disabled            |
| [Format support](FORMAT_SUPPORT.md)               | Decoder, viewer and platform limits                                  |
| [Release status](RELEASE_STATUS.md)               | Distribution limits and outstanding acceptance gates                 |
| [Preservation contract](PRESERVATION_CONTRACT.md) | Binding source-file and recovery rules                               |

## Work on the application

| Document                                                     | Purpose                                                         |
| ------------------------------------------------------------ | --------------------------------------------------------------- |
| [Development](DEVELOPMENT.md)                                | Setup, commands, focused validation and native test isolation   |
| [Architecture](ARCHITECTURE.md)                              | Runtime boundaries, catalog ownership and experimental features |
| [TypeScript and Chakra conventions](TYPESCRIPT_STRUCTURE.md) | Component layout, styling and form contracts                    |
| [Dependency inventory](DEPENDENCIES.md)                      | Pins, helper manifests, notices and advisory checks             |
| [Verification](ACCEPTANCE_TESTS.md)                          | Implemented behavior and the tests that exercise it             |
| [Decisions](DECISIONS.md)                                    | Current architecture choices and preserved rationale            |
| [Vocabulary](CONTEXT.md)                                     | Library, preview and reconciliation terminology                 |
| [Product identity](PRODUCT_IDENTITY.md)                      | Name, logo and persisted-identity rules                         |
| [Implementation log](IMPLEMENTATION_LOG.md)                  | Changes recorded after the documentation cleanup                |

## Historical material

The root [product](../PRODUCT.md) and [design](../DESIGN.md) summaries also reflect
the implemented build. The [changelog archive](changelog/README.md) holds previous plans, prompts,
milestone tasks, specifications, architecture records and dated test reports.
Those pages preserve their original claims and limitations; they are not current
setup instructions, newly passing evidence or permission to enable a feature.
Archiving a requirement does not satisfy or waive a preservation or release gate.
