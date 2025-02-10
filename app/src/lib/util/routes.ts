export type NavigationRoute =
    | "/"
    | "/init";

export function parse_route(route: string): NavigationRoute {
    switch (route) {
        case "/":
        case "/init":
            return route;
        default:
            return "/";
    };
};