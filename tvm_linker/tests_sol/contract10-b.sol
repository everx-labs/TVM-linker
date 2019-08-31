pragma solidity ^0.5.0;

import "contract10.sol";

// Tests for sending and receiving arrays

contract Receiver is IReceiver {

	uint m_counter;
	uint m_sum1;

	function on_uint64(uint64[] memory arr) public {
		uint sum = 0;
		uint len = arr.length;
		for (uint i = 0; i < len; i++) {
			sum += arr[i];
		}
		m_counter = sum;
	}

	function on_two_uint64(uint64[] memory arr0, uint64[] memory arr1) public {
		m_counter = 0;
		for (uint i = 0; i < arr0.length; i++) {
			m_counter += arr0[i];
		}
		m_sum1 = 0;
		for (uint i = 0; i < arr1.length; i++) {
			m_sum1 += arr1[i];
		}
	}
}
