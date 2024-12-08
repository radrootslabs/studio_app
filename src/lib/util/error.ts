import type { ErrorMessage } from "@radroots/utils";

export const throw_err = (param: string | ErrorMessage<string>): undefined => {
    if (typeof param === `string`) throw new Error(param);
    else throw new Error(param.err);
};



