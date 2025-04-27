export type NavigationRoute =
    | "/"
    | "/farm"
    | "/profile"
    | "/profile/edit"
    | "/init";

export function parse_route(route: string): NavigationRoute {
    switch (route) {
        case "/":
        case "/farm":
        case "/profile":
        case "/profile/edit":
        case "/init":
            return route;
        default:
            return "/";
    };
};