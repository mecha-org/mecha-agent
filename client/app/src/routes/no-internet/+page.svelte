<script lang="ts">
	import NoInternetFound from '$lib/images/gifs/NoInternetFound.gif';
	import Layout from '../../shared/layout.svelte';
	import Icons from '../../shared/Icons.svelte';

	const goBack = () => {
		history.back();
	};
</script>

<Layout>
	<div class="flex flex-col" style="height:-webkit-fill-available">
		<div class="flex flex-grow flex-col items-center justify-center gap-2">
			<div>
				<img alt="check internet" src={NoInternetFound} />
			</div>
			<div class="flex justify-center text-base">
				<span> Connect to Internet to complete the setup </span>
			</div>
		</div>
	</div>

	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div class="flex h-full w-full flex-row items-center justify-between px-4 py-3">
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-xl bg-[#2A2A2C] p-2 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="back_icon" width="32" height="32" />
			</button>
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-xl bg-[#2A2A2C] p-2 text-[#FAFBFC]"
				on:click={() => {}}
			>
				<Icons name="settings_icon" width="32" height="32" />
			</button>
		</div>
	</footer>
</Layout>
