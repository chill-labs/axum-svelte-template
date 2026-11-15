<script lang="ts">
  import { API_ENDPOINTS, apiRequest, isHotSupportActive } from '#lib/api/client.ts';
  import type { MediaFileResponse } from '#lib/api/types.generated.ts';
  import { deobfuscateBytes } from '#lib/utils/obfuscation.ts';

  interface Props {
    src?: string | null;
    xorKey?: string | null;
    entityType?: 'user' | 'workspace' | 'todo' | null;
    entityId?: number | null;
    mediaKey?: string | null;
    alt?: string;
    fallbackText?: string;
    class?: string;
  }

  let {
    src = null,
    xorKey = null,
    entityType = null,
    entityId = null,
    mediaKey = null,
    alt = '',
    fallbackText = '?',
    class: className = 'h-9 w-9 rounded-full object-cover'
  }: Props = $props();

  let resolvedUrl = $state<string | null>(null);
  let loadFailed = $state(false);

  $effect(() => {
    const currentSrc = src;
    const currentKey = xorKey;
    const currentEntityType = entityType;
    const currentEntityId = entityId;
    const currentMediaKey = mediaKey;
    loadFailed = false;

    let revoked = false;
    let objectUrl: string | null = null;
    const controller = new AbortController();

    async function loadProtected() {
      let targetSrc = currentSrc;
      let targetKey = currentKey;

      if (!targetKey && currentMediaKey && currentEntityType && currentEntityId) {
        const res = await apiRequest<MediaFileResponse>(API_ENDPOINTS.MEDIA_GET_ENTITY_MEDIA, {
          params: {
            entity_type: currentEntityType,
            entity_id: currentEntityId
          }
        });
        if (revoked) return;
        if (res.success && res.data) {
          targetSrc = res.data.public_url;
          targetKey = res.data.xor_key;
        } else if (!targetSrc) {
          resolvedUrl = null;
          return;
        }
      }

      if (!targetSrc) {
        resolvedUrl = null;
        return;
      }

      if (!targetKey) {
        resolvedUrl = targetSrc;
        return;
      }

      const headers: Record<string, string> = {};
      if (targetSrc.startsWith('/') && isHotSupportActive()) {
        headers['X-Hot-Support'] = '1';
      }

      const fetchRes = await fetch(targetSrc, {
        credentials: targetSrc.startsWith('/') ? 'same-origin' : 'omit',
        headers,
        signal: controller.signal
      });
      if (!fetchRes.ok) throw new Error('Media load failed');
      const buf = await fetchRes.arrayBuffer();
      const plain = deobfuscateBytes(new Uint8Array(buf), targetKey);
      const contentType = fetchRes.headers.get('content-type') || 'image/webp';
      const copy = new Uint8Array(plain.byteLength);
      copy.set(plain);
      const blob = new Blob([copy], { type: contentType });
      if (!revoked) {
        objectUrl = URL.createObjectURL(blob);
        resolvedUrl = objectUrl;
      }
    }

    loadProtected().catch(() => {
      if (!revoked) {
        loadFailed = true;
        resolvedUrl = null;
      }
    });

    return () => {
      revoked = true;
      controller.abort();
      if (objectUrl) {
        URL.revokeObjectURL(objectUrl);
      }
    };
  });

  const initials = $derived(fallbackText.trim().slice(0, 2).toUpperCase() || '?');
</script>

{#if resolvedUrl && !loadFailed}
  <img src={resolvedUrl} {alt} class={className} />
{:else}
  <div
    class="flex items-center justify-center bg-gradient-to-br from-blue-500 to-indigo-600 font-bold text-white {className}"
    aria-label={alt}
  >
    <span class="text-xs">{initials}</span>
  </div>
{/if}
