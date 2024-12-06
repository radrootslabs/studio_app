import type { ErrorMessage } from "@radroots/utils";

export const throw_err = (param: string | ErrorMessage<string>, cause: string = ``): void => {
    if (typeof param === `string`) throw new Error(param, { cause });
    else if (`err` in param) throw new Error(param.err, { cause });
};

export const catch_err = async (e: unknown, fn_name: string): Promise<void> => {
    if (e instanceof Error) {
        console.error(`(catch_err) `, e.name, e.message, e.stack, e.cause)
    } else {
        console.log(`(catch_err) `, e)
    }
    //@todo
};

