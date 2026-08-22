export async function handleDeepLink(urlStrings: string[]): Promise<void> {
    const [urlString] = urlStrings;
    if (!urlString) {
        return;
    }

    const url = new URL(urlString);

    console.debug("Deep link received", url);

    switch (url.pathname) {
        default: {
            console.warn(`Unknown deep link: ${urlString}`);
            break;
        }
    }
}
