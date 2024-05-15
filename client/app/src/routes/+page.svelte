<script lang="ts">
	import * as Carousel from '$lib/components/ui/carousel';
	import Timer from '$lib/images/svgs/Timer.svg';
	import Globe from '$lib/images/svgs/Globe.svg';
	import Encrypt from '$lib/images/svgs/Encrypt.svg';
	import Header from '$lib/custom-components/Header.svelte';
	import { goto } from '$app/navigation';
	import { invoke } from '@tauri-apps/api';
	import Layout from '../shared/layout.svelte';
	import Icons from '../Icons.svelte';
	import SubHeader from '$lib/custom-components/SubHeader.svelte';
	import Autoplay from 'embla-carousel-autoplay';
	import { check_machine_provision_status, get_machine_id } from '$lib/services';
	import { machineInfo } from '$lib/stores';
	import toast from 'svelte-french-toast';

	const goNext = async () => {
		try {
			const data = await check_machine_provision_status();
			
			if (data.status) {
				console.log('is_machine_provisioned ');
				try {
					const data = await get_machine_id();
					machineInfo.set({ id: data.machine_id });
					goto('/machine-info');

					// // TEMP - for test
					// goto('../check-internet');
					// goto('../configure-machine');
				} catch (error) {
					console.error('Error: Check machine id : ', error);
					toast.error('Agent is not running');
				}
			} else {
				goto('../check-internet');
			}
		} catch (error: any) {
			console.error('Error: Check machine provision : ', error);
			toast.error(error);
		}
	};

	const goBack = () => {
		invoke('exit_app');
	};
</script>

<Layout>
	<div class="flex flex-col">
		<Header title={'Connect to Mecha'} />

		<SubHeader text={"Please sign up on mecha.so before getting started."} />

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
						<div class="flex h-full w-full flex-col items-center rounded-md bg-[#15171D] p-10">
							<img src={Globe} alt="globe" />
							<!-- <Icons name="virtual_network_icon" class="" /> -->
							<p class="mt-8 text-center text-base font-medium">
								Virtual networking to enable connecting to your machine remotely
							</p>
						</div>
					</Carousel.Item>
					<Carousel.Item>
						<div class="flex h-full w-full flex-col items-center rounded-md bg-[#15171D] p-10">
							<img src={Timer} alt="timer" />
							<p class="mt-8 text-center text-base font-medium">
								Integrated Telemetry that collects logs and metrics in real-time
							</p>
						</div>
					</Carousel.Item>
					<Carousel.Item>
						<div class="flex h-full w-full flex-col items-center rounded-md bg-[#15171D] p-10">
							<img src={Encrypt} alt="encrypt" />
							<p class="mt-8 text-center text-base font-medium">
								Secure and encrypted messaging from-to your machine
							</p>
						</div>
					</Carousel.Item>
				</Carousel.Content>
			</Carousel.Root>
		</div>
	</div>
	<footer slot="footer" class="h-full w-full bg-[#05070A73] backdrop-blur-3xl backdrop-filter">
		<div class="flex h-full w-full flex-row items-center justify-between px-4 py-3">
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-md bg-[#15171D] p-2 text-[#FAFBFC]"
				on:click={goBack}
			>
				<Icons name="back_icon" width="32" height="32" />
			</button>
			<button
				class="flex h-[48px] w-[48px] items-center justify-center rounded-md bg-[#15171D] p-2 text-[#FAFBFC]"
				on:click={goNext}
			>
				<Icons name="next_icon" width="32" height="32" />
			</button>
		</div>
	</footer>
</Layout>
