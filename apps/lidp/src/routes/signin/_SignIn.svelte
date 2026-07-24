<script lang="ts" module>
	import * as v from "valibot";
	import { m } from "$lib/paraglide/messages";

	const SignInSchema = () =>
		v.object({
			username: v.pipe(
				v.string(),
				v.nonEmpty(m.errors_message_username_required()),
			),
			password: v.pipe(
				v.string(),
				v.nonEmpty(m.errors_message_password_required()),
				v.minLength(1, m.errors_message_password_min_length({ characters: 1 })),
			),
		});
</script>

<script lang="ts">
	import { createForm } from "@aicacia/svelte-forms";
	import Issues from "$lib/common/components/Issues.svelte";
	import { afterSigninRedirect } from "$lib/common/state/afterSignInRedirectPath.svelte";
	import { setTokenResponse } from "$lib/common/state/auth.svelte";
	import { lidpApi } from "$lib/common/state/lidpClient.svelte";

	const form = createForm(SignInSchema(), {
		username: "",
		password: "",
	});

	async function onSubmit(e: SubmitEvent) {
		e.preventDefault();

		const [_input, output, error] = await form.validate();

		if (error) {
			return;
		}
		const token = await lidpApi.token({
			grantType: "password",
			clientId: false ? "lidp-management-desktop" : "lidp-management-web",
			username: output.username,
			password: output.password,
			scope: "openid profile email",
		});

		setTokenResponse(token);
		await afterSigninRedirect();
	}
</script>

<form onsubmit={onSubmit} class="flex flex-col">
	<label class="flex flex-col">
		{m.signin_username_label()}
		<input
			type="text"
			aria-label={m.signin_username_label()}
			autocomplete="username"
			placeholder={m.signin_username_placeholder()}
			bind:value={form.fields.username.value}
		/>
		<Issues issues={form.fields.username.issues} />
	</label>
	<label class="flex flex-col">
		{m.signin_password_label()}
		<input
			aria-label={m.signin_password_label()}
			type="password"
			autocomplete="current-password"
			placeholder={m.signin_password_placeholder()}
			bind:value={form.fields.password.value}
		/>
		<Issues issues={form.fields.password.issues} />
	</label>
	<input class="btn primary mt-4" type="submit" value={m.sign_in()} />
</form>
