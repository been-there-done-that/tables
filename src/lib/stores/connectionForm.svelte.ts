import type { 
  DatabaseEngine, 
  ConnectionDraft, 
  ConnectionFormState, 
  EngineCapabilities,
  Connection,
  RuntimeConnection
} from '$lib/types/connection';
import { getEngineCapabilities, createEmptyDraft, setNestedValue, getNestedValue } from '$lib/types/connection';

// Core form state using runes
let draft = $state<ConnectionDraft | null>(null);
let errors = $state<Record<string, string>>({});
let isDirty = $state(false);

// Derived computed values using $derived
const engine = $derived(draft?.engine);
const is_valid = $derived(() => {
  if (!draft) return false;
  if (draft.name.trim() === '') return false;
  if (Object.keys(errors).length > 0) return false;
  return true;
});

const capabilities = $derived(() => 
  engine ? getEngineCapabilities(engine) : null
);

const formState = $derived((): ConnectionFormState | null => {
  if (!draft) return null;
  return {
    draft: draft,
    errors: errors,
    is_valid: is_valid()
  };
});

// Export functions to access derived values
export function getEngine() {
  return engine;
}

export function getIsValid() {
  return is_valid();
}

export function getCapabilities() {
  return capabilities();
}

export function getFormState() {
  return formState();
}

// Form actions
export function initializeForm(engineParam: DatabaseEngine, existingConnection?: Connection) {
  if (existingConnection) {
    // Load from existing connection
    try {
      const config = JSON.parse(existingConnection.config_json);
      draft = {
        name: existingConnection.name,
        engine: existingConnection.engine as DatabaseEngine,
        config
      };
    } catch (e) {
      console.error('Failed to parse existing connection config:', e);
      resetForm();
    }
  } else {
    // Create new draft
    draft = createEmptyDraft(engineParam);
  }
  errors = {};
  isDirty = false;
}

export function resetForm() {
  draft = null;
  errors = {};
  isDirty = false;
}

export function updateEngine(newEngine: DatabaseEngine) {
  const currentDraft = draft;
  if (currentDraft) {
    const newDraft = createEmptyDraft(newEngine);
    newDraft.name = currentDraft.name; // Preserve name
    draft = newDraft;
    errors = {};
    isDirty = true;
  }
}

export function updateName(name: string) {
  const currentDraft = draft;
  if (currentDraft) {
    draft = {
      ...currentDraft,
      name: name.trim()
    };
    isDirty = true;
    
    // Clear name error if present
    if (errors.name) {
      const newErrors = { ...errors };
      delete newErrors.name;
      errors = newErrors;
    }
  }
}

export function updateConfigField(fieldPath: string, value: any) {
  const currentDraft = draft;
  if (!currentDraft) return;

  const newConfig = JSON.parse(JSON.stringify(currentDraft.config));
  setNestedValue(newConfig, fieldPath, value);
  
  draft = {
    ...currentDraft,
    config: newConfig
  };
  isDirty = true;

  // Clear field error if present
  if (errors[fieldPath]) {
    const newErrors = { ...errors };
    delete newErrors[fieldPath];
    errors = newErrors;
  }

  // Re-validate dependent fields
  validateField(fieldPath, newConfig);
}

export function validateField(fieldPath: string, config?: any) {
  const currentDraft = draft;
  if (!currentDraft) return;

  const configToValidate = config || currentDraft.config;
  const currentCapabilities = getEngineCapabilities(currentDraft.engine);
  const newErrors = { ...errors };

  // Find the field definition
  let fieldDef = null;
  for (const section of currentCapabilities.config_schema.sections) {
    // Check if section should be shown
    if (section.condition && !section.condition(configToValidate)) {
      continue;
    }
    
    fieldDef = section.fields.find(f => f.id === fieldPath);
    if (fieldDef) break;
  }

  if (!fieldDef) return;

  const fieldValue = getNestedValue(configToValidate, fieldPath);
  
  // Required field validation
  if (fieldDef.required && (fieldValue === undefined || fieldValue === '' || fieldValue === null)) {
    newErrors[fieldPath] = `${fieldDef.label} is required`;
    errors = newErrors;
    return;
  }

  // Type-specific validation
  if (fieldValue !== undefined && fieldValue !== '' && fieldValue !== null) {
    switch (fieldDef.type) {
      case 'number':
        const numValue = Number(fieldValue);
        if (isNaN(numValue)) {
          newErrors[fieldPath] = `${fieldDef.label} must be a valid number`;
        } else if (fieldDef.validation) {
          if (fieldDef.validation.min !== undefined && numValue < fieldDef.validation.min) {
            newErrors[fieldPath] = `${fieldDef.label} must be at least ${fieldDef.validation.min}`;
          } else if (fieldDef.validation.max !== undefined && numValue > fieldDef.validation.max) {
            newErrors[fieldPath] = `${fieldDef.label} must be at most ${fieldDef.validation.max}`;
          }
        }
        break;
        
      case 'text':
        if (fieldDef.validation?.pattern) {
          const regex = new RegExp(fieldDef.validation.pattern);
          if (!regex.test(String(fieldValue))) {
            newErrors[fieldPath] = fieldDef.validation.message || `${fieldDef.label} is invalid`;
          }
        }
        break;
    }
  }

  // Remove error if validation passes
  if (!newErrors[fieldPath] && errors[fieldPath]) {
    delete newErrors[fieldPath];
  }

  errors = newErrors;
}

export function validateForm(): boolean {
  const currentDraft = draft;
  if (!currentDraft) return false;

  const currentCapabilities = getEngineCapabilities(currentDraft.engine);
  const newErrors: Record<string, string> = {};

  // Validate name
  if (!currentDraft.name.trim()) {
    newErrors.name = 'Connection name is required';
  }

  // Validate all visible fields
  for (const section of currentCapabilities.config_schema.sections) {
    // Check if section should be shown
    if (section.condition && !section.condition(currentDraft.config)) {
      continue;
    }

    for (const field of section.fields) {
      // Check if field should be shown
      if (field.condition && !field.condition(currentDraft.config)) {
        continue;
      }

      const fieldValue = getNestedValue(currentDraft.config, field.id);
      
      // Required validation
      if (field.required && (fieldValue === undefined || fieldValue === '' || fieldValue === null)) {
        newErrors[field.id] = `${field.label} is required`;
        continue;
      }

      // Type validation
      if (fieldValue !== undefined && fieldValue !== '' && fieldValue !== null) {
        switch (field.type) {
          case 'number':
            const numValue = Number(fieldValue);
            if (isNaN(numValue)) {
              newErrors[field.id] = `${field.label} must be a valid number`;
            } else if (field.validation) {
              if (field.validation.min !== undefined && numValue < field.validation.min) {
                newErrors[field.id] = `${field.label} must be at least ${field.validation.min}`;
              } else if (field.validation.max !== undefined && numValue > field.validation.max) {
                newErrors[field.id] = `${field.label} must be at most ${field.validation.max}`;
              }
            }
            break;
            
          case 'text':
            if (field.validation?.pattern) {
              const regex = new RegExp(field.validation.pattern);
              if (!regex.test(String(fieldValue))) {
                newErrors[field.id] = field.validation.message || `${field.label} is invalid`;
              }
            }
            break;
        }
      }
    }
  }

  errors = newErrors;
  return Object.keys(newErrors).length === 0;
}

export function getDraftForSubmission(): ConnectionDraft | null {
  if (!validateForm()) return null;
  
  const currentDraft = draft;
  if (!currentDraft) return null;

  // Return a copy of the draft
  return JSON.parse(JSON.stringify(currentDraft));
}

// Secret management
export async function storeSecret(secretType: string, secretValue: string): Promise<string> {
  const response = await fetch('/api/secrets', {
    method: 'POST',
    headers: {
      'Content-Type': 'application/json',
    },
    body: JSON.stringify({
      type: secretType,
      value: secretValue
    })
  });

  if (!response.ok) {
    throw new Error('Failed to store secret');
  }

  const data = await response.json();
  return data.key_ref;
}

export async function getSecrets(): Promise<Array<{ key_ref: string; type: string; created_at: number }>> {
  const response = await fetch('/api/secrets');
  if (!response.ok) {
    throw new Error('Failed to fetch secrets');
  }
  return response.json();
}

export async function deleteSecret(keyRef: string): Promise<void> {
  const response = await fetch(`/api/secrets/${keyRef}`, {
    method: 'DELETE'
  });
  
  if (!response.ok) {
    throw new Error('Failed to delete secret');
  }
}

// Utility function to get field value for binding
export function getFieldValue(fieldPath: string): any {
  const currentDraft = draft;
  if (!currentDraft) return undefined;
  return getNestedValue(currentDraft.config, fieldPath);
}

// Utility function to check if field should be shown
export function isFieldVisible(fieldPath: string): boolean {
  const currentDraft = draft;
  if (!currentDraft) return false;

  const currentCapabilities = getEngineCapabilities(currentDraft.engine);
  
  for (const section of currentCapabilities.config_schema.sections) {
    // Check if section should be shown
    if (section.condition && !section.condition(currentDraft.config)) {
      continue;
    }

    const field = section.fields.find(f => f.id === fieldPath);
    if (field) {
      // Check if field should be shown
      if (field.condition && !field.condition(currentDraft.config)) {
        return false;
      }
      return true;
    }
  }

  return false;
}

// Utility function to get field error
export function getFieldError(fieldPath: string): string | undefined {
  return errors[fieldPath];
}
