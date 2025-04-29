export type NavigationRoute =
    | "/"
    | "/farms"
    | "/farms/add"
    | "/farms/details"
    | "/farms/products/add"
    | "/profile"
    | "/profile/edit"
    | "/init";

export function parse_route(route: string): NavigationRoute {
    switch (route) {
        case "/":
        case "/farms":
        case "/farms/add":
        case "/farms/details":
        case "/farms/products/add":
        case "/profile":
        case "/profile/edit":
        case "/init":
            return route;
        default:
            return "/";
    };
};