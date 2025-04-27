export type NavigationRoute =
    | "/"
    | "/farms"
    | "/farms/add"
    | "/profile"
    | "/profile/edit"
    | "/init";

export function parse_route(route: string): NavigationRoute {
    switch (route) {
        case "/":
        case "/farms":
        case "/farms/add":
        case "/profile":
        case "/profile/edit":
        case "/init":
            return route;
        default:
            return "/";
    };
};