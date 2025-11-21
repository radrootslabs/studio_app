export type NavigationRoute =
    | "/"
    | "/farms"
    | "/farms/add"
    | "/farms/info"
    | "/import"
    | "/profile"
    | "/profile/edit"
    | "/setup"

export function parse_route(route: string): NavigationRoute {
    switch (route) {
        case "/":
        case "/farms":
        case "/farms/add":
        case "/farms/info":
        case "/import":
        case "/profile":
        case "/profile/edit":
        case "/setup":
            return route;
        default:
            return "/";
    }
};