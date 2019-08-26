pragma solidity ^0.5.0;

import "contract10.sol";

// Tests for sending and receiving arrays

contract Receiver is IReceiver {

	uint m_counter;

	function on_uint64(uint64[] memory arr) public {
		uint sum = 0;
		uint len = arr.length;
		for (uint i = 0; i < len; i++) {
			sum += arr[i];
		}
		m_counter = sum;
	}
	
}
