pragma solidity ^0.5.0;

// Tests for sending and receiving arrays

contract IReceiver {
	function on_uint64(uint64[] memory arr) public;
}
