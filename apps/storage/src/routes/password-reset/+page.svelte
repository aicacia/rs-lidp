<script lang="ts" module>
	import * as v from "valibot";
	import { m } from "$lib/paraglide/messages";

	export const PasswordResetSchema = () =>
		v.pipe(
			v.object({
				newPassword: v.pipe(
					v.string(),
					v.nonEmpty(m.errors_message_password_required()),
					v.minLength(
						8,
						m.errors_message_password_min_length({ characters: 8 }),
					),
				),
				confirmPassword: v.pipe(
					v.string(),
					v.nonEmpty(m.errors_message_password_required()),
					v.minLength(
						8,
						m.errors_message_password_min_length({ characters: 8 }),
					),
				),
			}),
			v.check(
				(input) => input.newPassword === input.confirmPassword,
				m.errors_message_password_mismatch?.() ?? "Passwords do not match",
			),
		);
</script>

<script lang="ts">
	import { createForm } from "@aicacia/svelte-forms";
	import Issues from "$lib/common/components/Issues.svelte";
	import { afterSigninRedirect } from "$lib/common/state/afterSignInRedirectPath";
	import {
		requiresPasswordReset,
		resetPassword,
	} from "$lib/common/state/auth.svelte";

	let submitError: string | null = null;

	const form = createForm(PasswordResetSchema(), {
		newPassword: "",
		confirmPassword: "",
	});

	async function onSubmit(event: SubmitEvent) {
		event.preventDefault();
		submitError = null;

		const [_input, output, error] = await form.validate();
		if (error) return;

		try {
			await resetPassword(output.newPassword, output.confirmPassword);
			// Only redirect once the server flag is cleared locally
			if (!requiresPasswordReset()) {
				await afterSigninRedirect();
			}
		} catch (e) {
			submitError =
				e instanceof Error
					? e.message
					: (m.errors_message_unexpected?.() ?? "Unexpected error");
		}
	}
</script>

<div class="flex grow flex-col items-center justify-center">
	<div class="card w-sm">
		<h3>{m.profile_change_password_title()}</h3>
		<p class="mt-2 text-sm text-gray-600 dark:text-gray-300">
			{m.profile_change_password_description?.() ?? ""}
		</p>

		<form onsubmit={onSubmit} class="mt-4 flex flex-col gap-3">
			<label class="flex flex-col">
				<span>{m.profile_new_password_label()}</span>
				<input
					id="new-password"
					type="password"
					autocomplete="new-password"
					placeholder={m.profile_new_password_placeholder?.() ?? ""}
					aria-label={m.profile_new_password_label()}
					bind:value={form.fields.newPassword.value}
				/>
				<Issues issues={form.fields.newPassword.issues} />
			</label>

			<label class="flex flex-col">
				<span>{m.profile_confirm_password_label()}</span>
				<input
					id="confirm-password"
					type="password"
					autocomplete="new-password"
					placeholder={m.profile_confirm_password_placeholder?.() ?? ""}
					aria-label={m.profile_confirm_password_label()}
					bind:value={form.fields.confirmPassword.value}
				/>
				<Issues issues={form.fields.confirmPassword.issues} />
			</label>

			{#if submitError}
				<div class="alert danger">{submitError}</div>
			{/if}

			<input
				class="btn primary mt-2"
				type="submit"
				value={m.profile_save_password?.() ?? m.save?.() ?? "Save"}
			/>
		</form>
	</div>
</div>
