<script lang="ts">
	import * as Carousel from '$lib/components/ui/carousel';
	import Header from '../shared/Header.svelte';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api';
	import Layout from '../shared/layout.svelte';
	import Icons from '../shared/Icons.svelte';
	// import SubHeader from '../shared/SubHeader.svelte';
	import Autoplay from 'embla-carousel-autoplay';
	import { check_machine_provision_status, get_machine_id } from '$lib/services';
	import { machineInfo } from '$lib/stores';

	const goNext = async () => {
		try {
			const data = await check_machine_provision_status();

			if (data.status) {
				console.log('is_machine_provisioned ', data);
				try {
					const data = await get_machine_id();
					console.log('get_machine_id data: ', data);
					machineInfo.set({ id: data.machine_id });
					goto('/machine-info');
				} catch (error) {
					console.error('Error: Check machine id : ', error);
					goto('/setup-failed', { state: { error: error } });
				}
			} else {
				goto('../check-internet');
			}
		} catch (error: any) {
			console.error('Error: Check machine provision : ', error);
			goto('/setup-failed', { state: { error: error } });
		}
	};

	const goBack = () => {
		invoke('exit_app');
	};
</script>

<Layout>
	<div class="flex flex-col">
		<Header title={'Connect to Mecha'} />
		<div class="flex flex-grow flex-col">
			<Carousel.Root
				class="mx-auto w-full flex-grow "
				plugins={[
					Autoplay({
						delay: 3000
					})
				]}
			>
				<Carousel.Content>
					<Carousel.Item>
						<div class="flex h-full w-full flex-col items-center rounded-md p-8">
							<Icons name="virtual_network_icon" class="w-80 h-20" />
							<p class="mt-8 text-center text-xl font-medium">
								Virtual networking for remote access
							</p>
						</div>
					</Carousel.Item>
					<Carousel.Item>
						<div class="flex h-full w-full flex-col items-center rounded-md p-8">
							<Icons name="telemetry_icon" class="w-80 h-20" />
							<p class="mt-8 text-center text-xl font-medium">Integrated Telemetry real-time</p>
						</div>
					</Carousel.Item>
					<Carousel.Item>
						<div class="flex h-full w-full flex-col items-center rounded-md p-8">
							<Icons name="encypt_icon" class="w-80 h-20" />
							<p class="mt-8 text-center text-xl font-medium">Secure and encrypted messaging</p>
						</div>
					</Carousel.Item>
				</Carousel.Content>
			</Carousel.Root>
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
				on:click={goNext}
			>
				<Icons name="right_arrow" width="60" height="60" />
			</button>
		</div>
	</footer>
</Layout>
