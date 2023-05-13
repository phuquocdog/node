#![cfg_attr(not(feature = "std"), no_std)]


#[cfg(test)]
mod mock;

#[cfg(test)]
mod tests;

#[cfg(feature = "runtime-benchmarks")]
mod benchmarking;

#[frame_support::pallet]
pub mod pallet {
	use std::convert::TryInto;
	use sp_std::vec::Vec;
	use frame_support::pallet_prelude::*;
	use frame_system::pallet_prelude::*;

	pub type Amount = u128;
	pub type CurrencyId<T> = BoundedVec<u8, <T as Config>::MaxLength>;

	#[derive(TypeInfo,Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
	#[scale_info(skip_type_params(T))]

	pub struct CurrencyInfo<T: Config>{
		pub decimal: u16,
		pub rpc_url: BoundedVec<u8, T::MaxLength>,
		pub native: bool,
	}
	#[derive(TypeInfo,Encode, Decode, Clone, Default, RuntimeDebug, PartialEq, Eq)]
	pub struct CurrencyBalance {
		pub free: u128,
	}

	#[pallet::pallet]
	#[pallet::without_storage_info]
	pub struct Pallet<T>(_);
	/// Configure the pallet by specifying the parameters and types on which it depends.
	#[pallet::config]
	pub trait Config: frame_system::Config {
		/// Because this pallet emits events, it depends on the runtime's definition of an event.
		type RuntimeEvent: From<Event<Self>> + IsType<<Self as frame_system::Config>::RuntimeEvent>;
		/// The minimum length a name may be.
		#[pallet::constant]
		type MinLength: Get<u32>;

		/// The maximum length a name may be.
		#[pallet::constant]
		type MaxLength: Get<u32>;
	}

	// The pallet's runtime storage items.
	// https://docs.substrate.io/v3/runtime/storage
	// Learn more about declaring storage items:
	// https://docs.substrate.io/v3/runtime/storage#declaring-storage-items
	#[pallet::storage]
	#[pallet::getter(fn accounts)]
	pub type Accounts<T: Config> = StorageDoubleMap<
		_,
		Blake2_128Concat,
		T::AccountId,
		Blake2_128Concat,
		CurrencyId<T>,
		CurrencyBalance
	>;
	
	#[pallet::storage]
	#[pallet::getter(fn currency)]
	pub type Currency<T: Config> = StorageMap<
		_,
		Blake2_128Concat,
		CurrencyId<T>,
		CurrencyInfo<T>
	>;

	#[pallet::storage]
	#[pallet::getter(fn currencies)]
	pub(super) type Currencies<T: Config> = StorageValue<_, Vec<BoundedVec<u8, T::MaxLength>>>;
	

	// Pallets use events to inform users when important changes are made.
	// https://docs.substrate.io/v3/runtime/events-and-errors
	#[pallet::event]
	#[pallet::generate_deposit(pub(super) fn deposit_event)]
	pub enum Event<T: Config> {
		/// Event documentation should end with an array that provides descriptive names for event
		/// parameters. [something, who]
		/// Currency add success. [currency_id, who]
		NewCurrencyAdded(CurrencyId<T>, T::AccountId),
		/// Update balance success. [currency_id, amount, to, who]
		BalanceUpdated(CurrencyId<T>, Amount, T::AccountId, T::AccountId),
		/// Currency transfer success. [currency_id, amount, to, who]
		Transferred(CurrencyId<T>, Amount, T::AccountId, T::AccountId),
	}

	// Errors inform users that something went wrong.
	#[pallet::error]
	pub enum Error<T> {
		CurrencyExist,
		CurrencyNotExist,
		BadOrigin,
		InsufficientFunds,
		OverflowAmount,
		InsufficientAmount,
		TooLong
	}

	// Dispatchable functions allows users to interact with the pallet and invoke state changes.
	// These functions materialize as "extrinsics", which are often compared to transactions.
	// Dispatchable functions must be annotated with a weight and must return a DispatchResult.
	#[pallet::call]
	impl<T: Config> Pallet<T> {
		/// An example dispatchable that takes a singles value as a parameter, writes the value to
		/// storage and emits an event. This function must be dispatched by a signed extrinsic.
		#[pallet::call_index(0)]
		#[pallet::weight({50_000_000})]
		pub fn add_currency(
			origin: OriginFor<T>,
			currency_id: Vec<u8>,
			decimal: u16,
			rpc_url: Vec<u8>,
			native: bool,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let currency_id: BoundedVec<_, _> = currency_id.try_into().map_err(|_| Error::<T>::TooLong)?;
			let currency = Self::currency(currency_id.clone());
			let rpc_url: BoundedVec<_, _> =
				rpc_url.try_into().map_err(|_| Error::<T>::TooLong)?;
			let currency_info = CurrencyInfo { decimal, rpc_url, native };

			ensure!(currency.is_none(), Error::<T>::CurrencyExist);

			let mut currencies = Self::currencies().unwrap_or_default();
			currencies.push(currency_id.clone());

			Currency::<T>::insert(&currency_id, &currency_info);
			Currencies::<T>::put(currencies);

			Self::deposit_event(Event::NewCurrencyAdded(currency_id, who));

			Ok(())
		}
		#[pallet::call_index(1)]
		#[pallet::weight({60_000_000})]
		pub fn update_balance(
			origin: OriginFor<T>,
			to: T::AccountId,
			currency_id: CurrencyId<T>,
			amount: u128,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let currency = Self::currency(currency_id.clone());

			ensure!(currency.is_some(), Error::<T>::CurrencyNotExist);
			ensure!(amount > 0, Error::<T>::InsufficientAmount);

			let receiver_balance =
				Self::accounts(&to, &currency_id).unwrap_or(CurrencyBalance { free: 0 });
			let updated_to_balance =
				receiver_balance.free.checked_add(amount).ok_or(Error::<T>::OverflowAmount)?;

			Accounts::<T>::insert(&to, &currency_id, CurrencyBalance { free: updated_to_balance });

			Self::deposit_event(Event::BalanceUpdated(currency_id, amount, to, who));

			Ok(())
		}

		/// Transfer tokens from one account to another
		//#[pallet::weight(T::WeightInfo::transfer(currency_id.len() as u32 ))]
		#[pallet::call_index(2)]
		#[pallet::weight({70_000_000})]
		pub fn transfer(
			origin: OriginFor<T>,
			to: T::AccountId,
			currency_id: CurrencyId<T>,
			amount: u128,
		) -> DispatchResult {
			let who = ensure_signed(origin)?;
			let currency = Self::currency(currency_id.clone());

			ensure!(currency.is_some(), Error::<T>::CurrencyNotExist);
			ensure!(who != to, Error::<T>::BadOrigin);
			ensure!(amount > 0, Error::<T>::InsufficientAmount);

			let sender_balance =
				Self::accounts(&who, &currency_id).unwrap_or(CurrencyBalance { free: 0 });
			let receiver_balance =
				Self::accounts(&to, &currency_id).unwrap_or(CurrencyBalance { free: 0 });

			// Calculate new balances
			let updated_from_balance =
				sender_balance.free.checked_sub(amount).ok_or(Error::<T>::InsufficientFunds)?;
			let updated_to_balance =
				receiver_balance.free.checked_add(amount).ok_or(Error::<T>::OverflowAmount)?;

			// Write new balances to storage
			Accounts::<T>::insert(
				&who,
				&currency_id,
				CurrencyBalance { free: updated_from_balance },
			);
			Accounts::<T>::insert(&to, &currency_id, CurrencyBalance { free: updated_to_balance });

			Self::deposit_event(Event::Transferred(currency_id, amount, to, who));

			Ok(())
		}
	}
}
