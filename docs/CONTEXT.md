# Library vocabulary

| Term                 | Meaning                                                                                                                                                                                 |
| -------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Registered root      | A selected photo folder indexed under its saved scope. Its availability can change independently of its retained contents.                                                              |
| Catalog observation  | Stillharbour's last known information about a file or folder. Retained observations are not proof that originals are currently accessible.                                              |
| Invalidation hint    | A notification that contents may have changed. It requests reconciliation rather than establishing a deletion or move.                                                                  |
| Reconciliation       | Read-only comparison of an accessible library scope with previous observations. Incomplete or inaccessible scopes retain uncertainty.                                                   |
| Observed change      | A difference found between observations, such as an added location or an unambiguous move. It does not cover every transient filesystem event or establish integrity/recovery evidence. |
| Missing location     | A previously observed location absent from a successfully enumerated scope. Its contents may have moved elsewhere; this does not prove physical deletion.                               |
| Offline root         | A temporarily unavailable library whose last known contents remain retained. Offline does not mean empty.                                                                               |
| Resource reservation | A temporary allocation of a bounded work budget. It limits admitted work but does not prove the operating system or a helper stayed below a physical resource ceiling.                  |
| Source volume group  | Locations known to belong to the same native filesystem volume. Distinct groups do not prove that their underlying physical devices or network servers are independent.                 |
