<script lang="ts">
  import Button from "./Button.svelte";

  let {
    onCancel,
    onTest,
    onSave,
    cancelLabel = "Cancel",
    testLabel = "Test connection",
    saveLabel = "Save",
    showTest = true,
  } = $props<{
    onCancel?: () => void;
    onTest?: () => void;
    onSave?: () => void;
    cancelLabel?: string;
    testLabel?: string;
    saveLabel?: string;
    showTest?: boolean;
  }>();
</script>

<footer class="flex items-center justify-between pt-4 border-t border-(--theme-border-default) px-6 pb-4">
  <div></div>
  <div class="flex gap-2">
    <Button variant="outline" onClick={onCancel}>
      {cancelLabel}
    </Button>

    {#if showTest}
      <Button variant="outline" onClick={onTest}>
        {testLabel}
      </Button>
    {/if}

    <Button variant="solid" onClick={onSave}>
      {saveLabel}
    </Button>
  </div>
</footer>
