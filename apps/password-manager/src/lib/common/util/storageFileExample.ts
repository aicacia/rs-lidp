import {
    readStorageFile,
    type StorageClient,
    writeStorageFile,
} from "@aicacia/storage-client";

export interface StorageFileExampleState {
    status: "idle" | "writing" | "reading" | "success" | "error";
    message: string;
    content?: string;
}

/**
 * Performs a live storage file write/read demo.
 * Writes an example file to the storage bridge, then reads it back.
 */
export async function runStorageFileExample(
    client: StorageClient,
): Promise<StorageFileExampleState> {
    try {
        const testPath = "example/hello.txt";
        const testContent = "Hello from storage!";

        // Write the file
        await writeStorageFile(client, testPath, testContent);

        // Read the file back
        const readContent = await readStorageFile(client, testPath);

        // Verify the content matches
        if (readContent === testContent) {
            return {
                status: "success",
                message: `Successfully wrote and read file: ${testPath}`,
                content: readContent,
            };
        } else {
            return {
                status: "error",
                message: `Content mismatch: expected "${testContent}", got "${readContent}"`,
                content: readContent,
            };
        }
    } catch (error) {
        return {
            status: "error",
            message: `Storage operation failed: ${error instanceof Error ? error.message : String(error)}`,
        };
    }
}
