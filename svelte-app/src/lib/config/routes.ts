// =============================================================================
// ROUTE REGISTRY & SECURITY CLASSIFICATION (FRONTEND MIRROR)
// =============================================================================
// Mirrors `crates/shared/src/config/routes.rs` so client navigation guards match
// server access policies.

export type RouteType = 'public' | 'guest_only' | 'protected';

export interface RouteMeta {
  path: string;
  type: RouteType;
  adminOnly?: boolean;
}

export const ROUTES = {
  HOME: '/',
  TODOS: '/todos',
  WORKSPACES: '/workspaces',
  WORKSPACE_MEMBERS: '/workspaces/members',
  PROFILE: '/profile',
  SETTINGS: '/settings',
  ADMIN: '/admin',
  LOGIN: '/auth/login',
  REGISTER: '/auth/register',
  LOGIN_PASSWORD: '/auth/login-password',
  LOGIN_PASSWORD_ALIAS: '/login-password',
  ONBOARDING: '/auth/onboarding',
  RESET_PASSWORD: '/auth/reset-password',
  RESET_SUCCESS: '/auth/reset-success',
  HEALTHCHECK: '/healthcheck',
  TERMS: '/terms',
  PRIVACY: '/privacy'
} as const;

export const ROUTE_RULES: RouteMeta[] = [
  { path: ROUTES.HOME, type: 'public' },
  { path: ROUTES.TODOS, type: 'public' },
  { path: ROUTES.HEALTHCHECK, type: 'public' },
  { path: ROUTES.TERMS, type: 'public' },
  { path: ROUTES.PRIVACY, type: 'public' },
  { path: ROUTES.LOGIN, type: 'guest_only' },
  { path: ROUTES.REGISTER, type: 'guest_only' },
  { path: ROUTES.LOGIN_PASSWORD, type: 'guest_only' },
  { path: ROUTES.LOGIN_PASSWORD_ALIAS, type: 'guest_only' },
  { path: ROUTES.ONBOARDING, type: 'protected' },
  { path: ROUTES.RESET_PASSWORD, type: 'protected' },
  { path: ROUTES.RESET_SUCCESS, type: 'protected' },
  { path: ROUTES.WORKSPACES, type: 'protected' },
  { path: ROUTES.WORKSPACE_MEMBERS, type: 'protected' },
  { path: ROUTES.PROFILE, type: 'protected' },
  { path: ROUTES.SETTINGS, type: 'protected' },
  { path: ROUTES.ADMIN, type: 'protected', adminOnly: true }
];

export function matchRouteRule(pathname: string): RouteMeta | null {
  const normalized = pathname.length > 1 ? pathname.replace(/\/+$/, '') : pathname;
  for (const rule of ROUTE_RULES) {
    if (rule.path === normalized) {
      return rule;
    }
  }
  if (normalized.startsWith('/workspaces/')) {
    return { path: normalized, type: 'protected' };
  }
  return null;
}
