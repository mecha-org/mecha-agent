<script lang="ts">
	import Failure from '$lib/images/gifs/Failure.gif';
	import { page } from '$app/stores';
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../Icons.svelte';

	const state: any = $page.state;
	const error: any = state.error;
	const goBack = () => {
		history.back();
	};
</script>

<Layout>
	<div class="flex flex-grow flex-col" style="height:-webkit-fill-available">
		<div class=" relative flex flex-grow flex-col items-center justify-center gap-2">
			<div>
				<img alt="setup failed" class="h-[200px] w-[200px]" src={Failure} />
			</div>
			<div class="flex flex-col items-center justify-center">
				<div class="text-lg">Error connecting to service</div>
				<div
					class="mt-2 rounded-md bg-gray-600 px-2 py-1 text-base font-bold capitalize text-red-500"
				>
					Error : {error}
				</div>
			</div>
		</div>
	</div>

	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div class="flex h-full w-full flex-row items-center justify-between px-4 py-3">
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-xl bg-[#15171D] p-2 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="back_icon" width="32" height="32" />
			</button>
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-xl bg-[#15171D] p-2 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="refresh_icon" width="32" height="32" />
			</button>
		</div>
	</footer>
</Layout>
