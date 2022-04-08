pragma solidity ^0.5.0;

contract IContract {
    function request(uint256 a, uint256 b, uint256 c) public;
}
contract TestBody {
	constructor() public {}
	function send(address a) public {
		IContract(a).request(1, 2, 3);
	}
}
