export type NavigationRoute =
    | "/"
    | "/import"
    | "/setup"

export function parse_route(route: string): NavigationRoute {
    switch (route) {
        case "/":
        case "/import":
        case "/setup":
            return route;
        default:
            return "/";
    }
};