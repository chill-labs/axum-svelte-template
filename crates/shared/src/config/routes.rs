// =============================================================================
// CENTRALIZED ROUTE DEFINITIONS
// =============================================================================
//
// Single source of truth for application paths, URL segments, auth gates, and
// role classification across Axum shell handling, middleware, and redirect rules.

/// Security classification for every frontend page route.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouteType {
    /// Accessible to all visitors (guests and authenticated users).
    Public,
    /// Intended for unauthenticated visitors; signed-in users redirect to `/todos`.
    GuestOnly,
    /// Requires a valid authenticated session (`auth-session` or `hot-support-session`).
    Protected,
}

/// Static metadata describing a route entry.
#[derive(Debug, Clone, Copy)]
pub struct Route {
    /// Full absolute path (used for server redirects and links).
    pub path: &'static str,
    /// Relative path segment.
    pub segment: &'static str,
    /// Human-readable default page title.
    pub title: &'static str,
    /// Access policy enforced by client & server middleware.
    pub route_type: RouteType,
    /// Whether admin role is required (`role == "admin"`).
    pub admin_only: bool,
}

/// Extracts the final non-empty segment of a static route path.
pub const fn extract_segment(path: &str) -> &str {
    let bytes = path.as_bytes();
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if bytes[i] == b'/' {
            let (_, tail) = path.split_at(i + 1);
            return tail;
        }
    }
    path
}

macro_rules! define_routes {
    (
        $(#[$struct_meta:meta])*
        pub struct $struct_name:ident {
            $(
                $(#[$field_meta:meta])*
                $name:ident => { path: $path:expr, title: $title:expr, route_type: $rt:expr, admin: $admin:expr }
            ),* $(,)?
        }
    ) => {
        $(#[$struct_meta])*
        pub struct $struct_name;

        impl $struct_name {
            $(
                $(#[$field_meta])*
                pub const $name: Route = Route {
                    path: $path,
                    segment: extract_segment($path),
                    title: $title,
                    route_type: $rt,
                    admin_only: $admin,
                };
            )*

            /// Returns all routes registered in this group.
            pub const fn all() -> &'static [Route] {
                &[$(Self::$name),*]
            }
        }
    };
}

define_routes! {
    /// Root landing route.
    pub struct HomeRoutes {
        INDEX => { path: "/", title: "Axum Svelte Todo", route_type: RouteType::Public, admin: false },
    }
}

define_routes! {
    /// Authentication, onboarding, and re-auth routes.
    pub struct AuthRoutes {
        LOGIN => { path: "/auth/login", title: "Sign In", route_type: RouteType::GuestOnly, admin: false },
        REGISTER => { path: "/auth/register", title: "Register", route_type: RouteType::GuestOnly, admin: false },
        LOGIN_PASSWORD => { path: "/auth/login-password", title: "Test Account Sign In", route_type: RouteType::GuestOnly, admin: false },
        LOGIN_PASSWORD_ALIAS => { path: "/login-password", title: "Test Account Sign In", route_type: RouteType::GuestOnly, admin: false },
        GOOGLE_CALLBACK => { path: "/auth/google/callback", title: "Google Sign In", route_type: RouteType::Public, admin: false },
        ONBOARDING => { path: "/auth/onboarding", title: "Complete Profile", route_type: RouteType::Protected, admin: false },
        RESET_PASSWORD => { path: "/auth/reset-password", title: "Security Verification", route_type: RouteType::Protected, admin: false },
        RESET_SUCCESS => { path: "/auth/reset-success", title: "Verification Complete", route_type: RouteType::Protected, admin: false },
    }
}

define_routes! {
    /// Main Todo application routes.
    pub struct TodoRoutes {
        INDEX => { path: "/todos", title: "Todos", route_type: RouteType::Public, admin: false },
    }
}

define_routes! {
    /// Workspace & collaborator management routes.
    pub struct WorkspaceRoutes {
        LIST => { path: "/workspaces", title: "Workspaces", route_type: RouteType::Protected, admin: false },
        MEMBERS => { path: "/workspaces/members", title: "Workspace Members", route_type: RouteType::Protected, admin: false },
    }
}

define_routes! {
    /// Account & preferences routes.
    pub struct AccountRoutes {
        PROFILE => { path: "/profile", title: "Profile", route_type: RouteType::Protected, admin: false },
        SETTINGS => { path: "/settings", title: "Settings", route_type: RouteType::Protected, admin: false },
    }
}

define_routes! {
    /// System administrator console routes.
    pub struct AdminRoutes {
        DASHBOARD => { path: "/admin", title: "Admin Console", route_type: RouteType::Protected, admin: true },
    }
}

define_routes! {
    /// Public operational and legal routes.
    pub struct SystemRoutes {
        HEALTHCHECK => { path: "/healthcheck", title: "Healthcheck", route_type: RouteType::Public, admin: false },
        TERMS => { path: "/terms", title: "Terms of Service", route_type: RouteType::Public, admin: false },
        PRIVACY => { path: "/privacy", title: "Privacy Policy", route_type: RouteType::Public, admin: false },
    }
}

/// Checks if a pathname matches any registered `Route` with `RouteType::Protected`.
pub fn is_protected_path(path: &str) -> bool {
    let trimmed = path.trim_end_matches('/');
    let normalized = if trimmed.is_empty() { "/" } else { trimmed };

    let groups: &[&[Route]] = &[
        HomeRoutes::all(),
        AuthRoutes::all(),
        TodoRoutes::all(),
        WorkspaceRoutes::all(),
        AccountRoutes::all(),
        AdminRoutes::all(),
        SystemRoutes::all(),
    ];

    for group in groups {
        for route in *group {
            if route.path == normalized {
                return route.route_type == RouteType::Protected;
            }
        }
    }

    normalized.starts_with("/workspaces/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protected_paths_classified_correctly() {
        assert!(is_protected_path("/workspaces"));
        assert!(is_protected_path("/workspaces/members"));
        assert!(is_protected_path("/admin"));
        assert!(is_protected_path("/profile"));
        assert!(is_protected_path("/settings"));
        assert!(!is_protected_path("/todos"));
        assert!(!is_protected_path("/auth/login"));
        assert!(!is_protected_path("/healthcheck"));
    }
}
