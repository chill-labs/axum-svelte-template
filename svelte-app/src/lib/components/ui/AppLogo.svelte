<script lang="ts">
  interface Props {
    sizeClass?: string;
  }

  let { sizeClass = 'h-8 w-8' }: Props = $props();
</script>

<img src="/logo/logo.svg" alt="Todo Workspace" class="{sizeClass} rounded-xl object-contain" />
