import { libraryData } from '../ipc';
import type { CatalogMetadataRequest, CatalogMetadataReply } from '../bindings';
type Kind = CatalogMetadataReply['kind'];
type Data<K extends Kind> =
  Extract<CatalogMetadataReply, { kind: K }> extends { data: infer D } ? D : never;
export async function metadata<K extends Kind>(
  request: CatalogMetadataRequest,
  kind: K,
): Promise<Data<K>> {
  const reply = await libraryData({ kind: 'catalog_metadata', request }, 'catalog_metadata');
  if (reply.kind !== kind)
    throw new Error('Unexpected catalog metadata response. Reopen the panel.');
  return reply.data as Data<K>;
}
