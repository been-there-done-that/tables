<script lang="ts">
  import FormInput from "../FormInput.svelte";

  let {
    onCancel,
    onSave,
    onTest,
  } = $props<{
    onCancel?: () => void;
    onSave?: (values: Record<string, string>) => void;
    onTest?: (values: Record<string, string>) => void;
  }>();

  const initial = { host: "", port: "", database: "", user: "", password: "" };
  let values = $state<Record<string, string>>({ ...initial });

  const setField = (key: string) => (e: Event) => {
    const target = e.currentTarget as HTMLInputElement | null;
    if (!target) return;
    values = { ...values, [key]: target.value };
  };

  export function triggerSave() {
    onSave?.(values);
  }

  export function triggerTest() {
    onTest?.(values);
  }

  export function triggerCancel() {
    onCancel?.();
  }
</script>

<div class="space-y-4">
  <!-- Host input -->
  <FormInput
    inputId="host"
    label="Host"
    placeholder="db.example.com"
    type="text"
    value={values.host ?? ""}
    oninput={setField("host")}
  />

  <!-- Port input -->
  <FormInput
    inputId="port"
    label="Port"
    placeholder="5432"
    type="number"
    value={values.port ?? ""}
    oninput={setField("port")}
  />

  <!-- Database input -->
  <FormInput
    inputId="database"
    label="Database"
    placeholder="postgres"
    type="text"
    value={values.database ?? ""}
    oninput={setField("database")}
  />

  <!-- User input -->
  <FormInput
    inputId="user"
    label="User"
    placeholder="admin"
    type="text"
    value={values.user ?? ""}
    oninput={setField("user")}
  />

  <!-- Password input -->
  <FormInput
    inputId="password"
    label="Password"
    placeholder="•••••••"
    type="password"
    value={values.password ?? ""}
    oninput={setField("password")}
  />

</div>
