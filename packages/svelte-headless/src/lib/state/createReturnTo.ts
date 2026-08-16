import { createStorage } from "./storage.svelte";

export type ReturnToOptions = {
    id: string;
    goto: (path: string) => Promise<void>;
};

export function createReturnTo({ id, goto }: ReturnToOptions) {
    const returnTo = createStorage<string | null>(`return-to:${id}`, null);

    function setURL(url: URL) {
        returnTo.item = url.toString().substring(url.origin.length);
    }

    function setPath(path: string) {
        setURL(new URL(path, location.origin));
    }

    async function onReturn() {
        const returnToPath = returnTo.item;

        if (returnToPath) {
          returnTo.item = null;
          try {
            await goto(returnToPath);
          } catch (error) {
            throw error;
          } finally {
            returnTo.item = returnToPath;
          }
        }
    }

    return {
        setURL,
        setPath,
        onReturn,
    };
}
