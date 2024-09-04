<script lang="ts">
	import NoNetwork from '$lib/images/gifs/NoNetwork.gif';
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../shared/Icons.svelte';
	import { goBack } from '$lib/services';
</script>

<Layout>
	<div class="flex flex-col" style="height:-webkit-fill-available">
		<div class=" relative flex flex-grow flex-col justify-end items-center gap-2">
			<div class="flex justify-center text-2xl">
				<span> No internet to complete the setup </span>
			</div>
			<div class="mt-2">
				<img alt="no internet" src={NoNetwork} />
			</div>
		</div>
	</div>

	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div
			class="border-silver-gray flex h-full w-full flex-row items-center justify-between border-t-2 px-4 py-3"
		>
			<button
				class="flex h-[60px] w-[60px] items-center justify-center rounded-lg p-1 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="left_arrow" width="60" height="60" />
			</button>
			<button
				class="flex h-[60px] w-[60px] items-center justify-center rounded-lg p-2 text-[#FAFBFC]"
				on:click={() => {}}
			>
				<Icons name="settings_icon" width="60" height="60" />
			</button>
		</div>
	</footer>
</Layout>
