/// <reference types="vite/client" />

interface ImportMetaEnv {
  /** `"0"` = Core-only UI (AIA-222); unset / other = AI Add-on enabled. */
  readonly VITE_AFM_AI_ADDON?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<{}, {}, any>;
  export default component;
}
