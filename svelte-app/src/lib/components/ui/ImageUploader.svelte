<script lang="ts">
  import { API_ENDPOINTS, apiRequest } from '#lib/api/client.ts';
  import type {
    MediaFileResponse,
    PresignUploadResponse,
    UploadPurpose
  } from '#lib/api/types.generated.ts';
  import Button from '#lib/components/ui/Button.svelte';
  import { uiPrefs } from '#lib/state/theme.svelte.ts';
  import { toasts } from '#lib/state/toast.svelte.ts';
  import { compressImageToWebp, obfuscateBytes } from '#lib/utils/obfuscation.ts';

  interface Props {
    purpose: UploadPurpose;
    entityId: number;
    label?: string;
    onUploaded: (media: MediaFileResponse) => void;
  }

  let {
    purpose,
    entityId,
    label,
    onUploaded
  }: Props = $props();

  let uploading = $state(false);
  let inputEl: HTMLInputElement | undefined = $state();

  async function handleFileChange(event: Event) {
    const target = event.currentTarget as HTMLInputElement;
    const file = target.files?.[0];
    if (!file) return;

    uploading = true;
    try {
      const { bytes, contentType } = await compressImageToWebp(file);
      const presign = await apiRequest<PresignUploadResponse>(
        API_ENDPOINTS.MEDIA_PRESIGN_UPLOAD,
        {
          body: {
            purpose,
            entity_id: entityId,
            content_type: contentType,
            file_size: bytes.byteLength
          }
        }
      );

      if (!presign.success || !presign.data) {
        toasts.error(uiPrefs.err(presign.message));
        return;
      }

      const encrypted = obfuscateBytes(bytes, presign.data.xor_key);
      const putBody = new Uint8Array(encrypted.byteLength);
      putBody.set(encrypted);

      const putRes = await fetch(presign.data.upload_url, {
        method: 'PUT',
        headers: { 'Content-Type': contentType },
        body: putBody
      });

      if (!putRes.ok) {
        toasts.error(uiPrefs.err('INTERNAL_ERROR'));
        return;
      }

      const confirmed = await apiRequest<MediaFileResponse>(
        API_ENDPOINTS.MEDIA_CONFIRM_UPLOAD,
        {
          body: {
            purpose,
            entity_id: entityId,
            storage_key: presign.data.storage_key,
            xor_key: presign.data.xor_key,
            content_type: contentType,
            file_size: encrypted.byteLength
          }
        }
      );

      if (confirmed.success && confirmed.data) {
        onUploaded(confirmed.data);
      } else {
        toasts.error(uiPrefs.err(confirmed.message));
      }
    } finally {
      uploading = false;
      target.value = '';
    }
  }
</script>

<div class="inline-flex items-center gap-2">
  <input
    bind:this={inputEl}
    type="file"
    accept="image/png,image/jpeg,image/webp"
    class="hidden"
    onchange={handleFileChange}
  />
  <Button
    variant="outline"
    size="sm"
    loading={uploading}
    onclick={() => inputEl?.click()}
  >
    {label ?? uiPrefs.dict.todos.uploadAttachment}
  </Button>
</div>
