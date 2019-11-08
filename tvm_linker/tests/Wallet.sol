pragma solidity ^0.5.0;

/// @title Simple wallet
/// @author Tonlabs
contract Wallet {
    /*
     *  Storage
     */
    uint256 owner;
    address public subscription;

/*
     Exception codes:
      100 - message sender is not a wallet owner.
      101 - limit is overrun.
      102 - invalid transfer value.
      103 - destination address is zero.
     */

    /*
     * Runtime functions
    */
    function tvm_sender_pubkey() private view returns (uint256) {}
    function tvm_logstr(bytes32 logstr) private view {}
    function tvm_transfer(address payable addr, uint128 value, bool bounce, uint16 flags) private {}
    function tvm_accept() private {}

    modifier checkOwnerAndAccept {
		require(tvm_sender_pubkey() == owner, 100);
        tvm_logstr("mod_accept");
        tvm_accept();
        _;
	}
    /*
     * Public functions
     */

    /// @dev Contract constructor.
    constructor() public {
        //TODO: tvm_accept();
        owner = tvm_sender_pubkey();
    }

    /// @dev Allows to transfer grams to destination account.
    /// @param dest Transfer target address.
    /// @param value Nanograms value to transfer.
    function sendTransaction(address payable dest, uint128 value, bool bounce) public checkOwnerAndAccept {
        tvm_logstr("sendTrans");
        require(value > 0 && value < address(this).balance, 102);
        tvm_logstr("func_accept");
        tvm_accept();
        tvm_transfer(dest, value, bounce, 0);
    }
    
}