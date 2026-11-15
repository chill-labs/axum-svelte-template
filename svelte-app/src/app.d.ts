import type { SessionBootstrapData } from '#lib/api/types.generated.ts';

export interface AppBootstrapPayload {
  session: SessionBootstrapData | null;
  lang: string;
  theme: 'light' | 'dark' | null;
  r2_public_url: string;
  is_hot_support: boolean;
}

declare global {
  interface Window {
    __APP_BOOTSTRAP__?: AppBootstrapPayload;
  }
}

export {};
